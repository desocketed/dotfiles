//! The GAME, CHAT and GLOBAL sinks: one `pw-loopback` each into the upstream sink (the default
//! sink at start, any headphones), their volumes set natively on the loopbacks' sink nodes.

use std::cell::RefCell;
use std::collections::HashMap;
use std::process::Child;
use std::process::Command;
use std::rc::Rc;

use anyhow::anyhow;
use novamix::Result;
use novamix::ipc::Output;
use novamix::ipc::Rule;
use novamix::ipc::Stream;
use novamix::mix::Mix;
use novamix::mix::Sink;
use novamix::mix::gain;
use pipewire::metadata::Metadata;
use pipewire::metadata::MetadataListener;
use pipewire::node::Node;
use pipewire::registry::Listener;
use pipewire::registry::RegistryRc;
use pipewire::spa::param::ParamType;
use pipewire::spa::pod::Object;
use pipewire::spa::pod::Pod;
use pipewire::spa::pod::Property;
use pipewire::spa::pod::PropertyFlags;
use pipewire::spa::pod::Value;
use pipewire::spa::pod::ValueArray;
use pipewire::spa::pod::serialize::PodSerializer;
use pipewire::spa::utils::SpaTypes;
use pipewire::types::ObjectType;

use crate::engine::Audio;
use crate::names::Names;
use crate::route;
use crate::state;

/// `--output default`: the same as no `--output`.
pub const DEFAULT: &str = "default";

/// `pw-loopback`'s default: stereo.
const CHANNELS: usize = 2;

/// A real sink the upstream can be.
struct Candidate {
    name: String,
    serial: u32,
    /// `priority.session`, then `priority.driver`: WirePlumber's own order.
    priority: (i64, i64),
}

struct Bound {
    sink: Sink,
    id: u32,
    serial: u32,
    node: Node,
}

/// A playback stream, before its link says where it plays.
struct StreamNode {
    id: u32,
    name: String,
    binary: String,
    props: HashMap<String, String>,
}

#[derive(Default)]
struct PwAudioInner {
    /// What the sinks are called.
    sinks: Names,
    /// The upstream sink, from `--output` or picked.
    target: Option<String>,
    /// No `--output`: the upstream is picked, and picked again when it goes away.
    follow: bool,
    /// How the upstream was chosen, for the log and `output`.
    how: String,
    /// The sink the user last settled on, from the state file or this run.
    remembered: Option<String>,
    /// The upstream is a stand-in for the remembered sink, which is not there: it goes back
    /// when that one reappears.
    standin: bool,
    /// The registry has listed what was there at start (a core sync came back).
    settled: bool,
    /// Every real sink that is not ours, by node id.
    candidates: HashMap<u32, Candidate>,
    loopbacks: Vec<Child>,
    nodes: Vec<Bound>,
    mix: Mix,
    /// The routing rules, to route the streams already playing once.
    rules: Vec<Rule>,
    /// Config `take_default`: make GLOBAL the default sink once the sinks are bound.
    take_default: bool,
    /// Set once the default sink has been taken (or not asked for).
    taken: bool,
    /// Set once the running streams have been routed: once per daemon start.
    swept: bool,
    /// Every node's `node.name`, to say where a link goes.
    names: HashMap<u32, String>,
    streams: Vec<StreamNode>,
    /// Link id to (output node, input node).
    links: HashMap<u32, (u32, u32)>,
    /// The default metadata, watched for the user moving a loopback's stream in a mixer.
    metadata: Option<(Metadata, MetadataListener)>,
}

/// A handle on the sinks; clones share them.
#[derive(Clone)]
pub struct PwAudio {
    inner: Rc<RefCell<PwAudioInner>>,
}
impl PwAudio {
    /// Creates the sinks at once when `output` is given or the default sink is a real one, else
    /// once the registry has been listed (see `settle`). The default sink at start is the
    /// user's: our sinks do not exist yet, so WirePlumber has not put GLOBAL in its place.
    pub fn new(
        output: Option<String>,
        sinks: Names,
    ) -> Result<Self> {
        let output = output.filter(|output| output != DEFAULT);
        let audio = Self {
            inner: Rc::new(RefCell::new(PwAudioInner {
                sinks: sinks.clone(),
                follow: output.is_none(),
                remembered: state::load(),
                ..PwAudioInner::default()
            })),
        };
        log_default(&sinks);
        match output {
            Some(output) => {
                log::info!("upstream is {output} (--output)");
                audio.inner.borrow_mut().how = "--output".into();
                audio.spawn(output);
            }
            None => match default_sink(&sinks) {
                Ok(name) => {
                    log::info!("upstream is {name} (the default sink at start)");
                    audio.inner.borrow_mut().how = "the default sink at start".into();
                    audio.remember(&name);
                    audio.spawn(name);
                }
                Err(err) => log::info!("{err}; picking the best real sink"),
            },
        }
        Ok(audio)
    }

    /// Remembers `name` as the user's sink, in memory and in the state file.
    fn remember(
        &self,
        name: &str,
    ) {
        self.inner.borrow_mut().remembered = Some(name.to_string());
        if let Err(err) = state::save(name) {
            log::warn!("cannot remember {name}: {err}");
        }
    }

    /// Called when the registry has listed what was there at start: with no upstream yet (the
    /// default was one of ours, or none), picks the remembered sink if it is there, else the
    /// highest-priority real sink.
    pub fn settle(&self) {
        {
            let mut inner = self.inner.borrow_mut();
            if inner.settled {
                return;
            }
            inner.settled = true;
        }
        self.pick("the default sink was not usable");
    }

    /// Spawns the loopbacks into the best candidate, if there is one and no upstream is set.
    fn pick(
        &self,
        why: &str,
    ) {
        let (name, how, remembered) = {
            let inner = self.inner.borrow();
            if !inner.follow || inner.target.is_some() {
                return;
            }
            match inner
                .remembered
                .as_ref()
                .filter(|remembered| inner.candidates.values().any(|c| &c.name == *remembered))
            {
                Some(name) => (Some(name.clone()), "the remembered sink", true),
                None => (best(&inner.candidates), "highest priority real sink", false),
            }
        };
        match name {
            Some(name) => {
                log::info!("upstream is {name} ({how}): {why}");
                {
                    let mut inner = self.inner.borrow_mut();
                    inner.how = how.into();
                    inner.standin = !remembered && inner.remembered.is_some();
                }
                // Nothing remembered yet: this is the user's sink until they say otherwise
                if !remembered && self.inner.borrow().remembered.is_none() {
                    self.remember(&name);
                }
                self.spawn(name);
            }
            None => log::warn!("no real sink to play to yet: {why}"),
        }
    }

    /// The remembered sink is back while a stand-in plays: moves the loopbacks to it.
    fn reattach(
        &self,
        name: &str,
    ) {
        {
            let inner = self.inner.borrow();
            if !inner.follow
                || !inner.standin
                || inner.target.is_none()
                || inner.remembered.as_deref() != Some(name)
            {
                return;
            }
        }
        log::info!("the remembered sink {name} is back");
        if let Err(err) = self.retarget(name, "the remembered sink, back") {
            log::warn!("cannot go back to {name}: {err}");
        }
    }

    /// Plays the three loopbacks to the candidate `name` without ending them: each loopback's
    /// playback stream gets `target.node` and `target.object` on the default metadata, as
    /// pipewire-pulse sets them for a move, so a mixer shows the sink as the stream's own choice
    /// (not "Default"), and WirePlumber relinks it. The sinks and what plays on them stay up.
    fn retarget(
        &self,
        name: &str,
        how: &str,
    ) -> Result<()> {
        let before = {
            let mut inner = self.inner.borrow_mut();
            // First, so the metadata events of our own writes are seen to agree
            let before = (inner.target.replace(name.to_string()), inner.how.clone(), inner.standin);
            inner.how = how.into();
            inner.standin = false;
            before
        };
        if let Err(err) = self.write_target(name) {
            let mut inner = self.inner.borrow_mut();
            (inner.target, inner.how, inner.standin) = before;
            return Err(err);
        }
        log::info!("upstream is {name} ({how})");
        Ok(())
    }

    /// Writes `name`'s node id and serial as the target of the three loopbacks' playback streams.
    fn write_target(
        &self,
        name: &str,
    ) -> Result<()> {
        let (id, serial, outputs) = {
            let inner = self.inner.borrow();
            let (id, candidate) = inner
                .candidates
                .iter()
                .find(|(_id, candidate)| candidate.name == name)
                .ok_or(anyhow!("{name} is not a real sink that is present"))?;
            let outputs: Vec<u32> = inner
                .names
                .iter()
                .filter(|(_id, node)| inner.sinks.is_output(node))
                .map(|(id, _node)| *id)
                .collect();
            (*id, candidate.serial, outputs)
        };
        if outputs.len() < Sink::ALL.len() {
            return Err(anyhow!("The sinks are not all up yet"));
        }
        for output in outputs {
            for (key, value) in [("target.node", id), ("target.object", serial)] {
                let status = Command::new("pw-metadata")
                    .args([&output.to_string(), key, &value.to_string(), "Spa:Id"])
                    .status()?;
                if !status.success() {
                    return Err(anyhow!("pw-metadata: {status}"));
                }
            }
        }
        Ok(())
    }

    /// The loopbacks' streams have just appeared (all three): says the upstream out loud on the
    /// metadata, so a mixer shows it.
    fn announce(&self) {
        let Some(name) = self.inner.borrow().target.clone() else {
            return;
        };
        if let Err(err) = self.write_target(&name) {
            log::debug!("not announcing {name} yet: {err}");
        }
    }

    /// A `target.object` on the default metadata changed. When it is on one of the loopbacks'
    /// streams and not what we wrote, a mixer moved it (pavucontrol): a real sink becomes the
    /// upstream, anything else is put back.
    fn moved(
        &self,
        subject: u32,
        value: Option<&str>,
    ) {
        enum Move {
            Real(String),
            Back(String),
        }
        let which = {
            let inner = self.inner.borrow();
            let Some(node) = inner.names.get(&subject).filter(|node| inner.sinks.is_output(node))
            else {
                return;
            };
            // Cleared when the node goes away
            let Some(serial) = value.and_then(|value| value.parse::<i64>().ok()) else {
                return;
            };
            let upstream = inner
                .target
                .as_ref()
                .and_then(|name| inner.candidates.values().find(|c| &c.name == name));
            if upstream.is_some_and(|c| i64::from(c.serial) == serial) {
                return;
            }
            let real = inner
                .candidates
                .values()
                .find(|candidate| i64::from(candidate.serial) == serial)
                .map(|candidate| candidate.name.clone());
            match real {
                Some(name) => Move::Real(name),
                // pipewire-pulse writes -1 for a move to the default sink
                None if serial < 0 => match default_node_name() {
                    Ok(name) if inner.candidates.values().any(|c| c.name == name) => {
                        Move::Real(name)
                    }
                    _ => Move::Back(format!("{node} was moved to the default sink, one of ours")),
                },
                None => Move::Back(format!("{node} was moved to a sink of novamix's own")),
            }
        };
        match which {
            Move::Real(name) => {
                if let Err(err) = self.retarget(&name, "the mixer") {
                    log::warn!("cannot follow the mixer to {name}: {err}");
                    return;
                }
                self.remember(&name);
                log::info!("upstream set from the mixer: {name}");
            }
            Move::Back(why) => {
                log::info!("{why}: moved back");
                self.announce();
            }
        }
    }

    /// The rules to apply once to the streams already playing, when the sinks are bound.
    pub fn with_rules(
        self,
        rules: Vec<Rule>,
    ) -> Self {
        self.inner.borrow_mut().rules = rules;
        self
    }

    /// Whether to make GLOBAL the default sink once the sinks are bound: the only way novamix
    /// ever changes the default.
    pub fn with_take_default(
        self,
        take: bool,
    ) -> Self {
        self.inner.borrow_mut().take_default = take;
        self
    }

    /// Watches the graph for the headset's sink and for the loopbacks' sink nodes.
    pub fn listen(
        &self,
        registry: &RegistryRc,
    ) -> Listener {
        let audio_ = self.clone();
        let audio__ = self.clone();
        let registry_ = registry.clone();
        registry
            .add_listener_local()
            .global(move |global| {
                let Some(props) = global.props else {
                    return;
                };
                if global.type_ == ObjectType::Link {
                    let node = |key| props.get(key)?.parse::<u32>().ok();
                    if let (Some(output), Some(input)) =
                        (node("link.output.node"), node("link.input.node"))
                    {
                        audio_
                            .inner
                            .borrow_mut()
                            .links
                            .insert(global.id, (output, input));
                    }
                    return;
                }
                if global.type_ == ObjectType::Metadata {
                    if props.get("metadata.name") == Some("default") {
                        audio_.watch(registry_.bind::<Metadata, _>(global));
                    }
                    return;
                }
                if global.type_ != ObjectType::Node {
                    return;
                }
                let Some(name) = props.get("node.name") else {
                    return;
                };
                {
                    let mut inner = audio_.inner.borrow_mut();
                    inner.names.insert(global.id, name.to_string());
                    // The loopbacks' own playback sides are streams too, but not the user's
                    let own = inner.sinks.is_output(name);
                    if !own && props.get("media.class") == Some("Stream/Output/Audio") {
                        inner.streams.push(StreamNode {
                            id: global.id,
                            name: props.get("application.name").unwrap_or(name).to_string(),
                            binary: props
                                .get("application.process.binary")
                                .unwrap_or_default()
                                .to_string(),
                            props: props
                                .iter()
                                .map(|(key, value)| (key.to_string(), value.to_string()))
                                .collect(),
                        });
                    }
                }
                let announce = {
                    let inner = audio_.inner.borrow();
                    inner.sinks.is_output(name)
                        && inner.names.values().filter(|n| inner.sinks.is_output(n)).count()
                            >= Sink::ALL.len()
                };
                if announce {
                    audio_.announce();
                }
                let sink = audio_.inner.borrow().sinks.sink_of(name);
                if sink.is_none() && props.get("media.class") == Some("Audio/Sink") {
                    let number = |key| props.get(key).and_then(|value| value.parse::<i64>().ok());
                    audio_.inner.borrow_mut().candidates.insert(
                        global.id,
                        Candidate {
                            name: name.to_string(),
                            serial: props
                                .get("object.serial")
                                .and_then(|serial| serial.parse().ok())
                                .unwrap_or_default(),
                            priority: (
                                number("priority.session").unwrap_or_default(),
                                number("priority.driver").unwrap_or_default(),
                            ),
                        },
                    );
                    // Seen after the start-up listing, with nothing to play to: take it
                    let settled = audio_.inner.borrow().settled;
                    if settled {
                        audio_.pick("the first real sink to appear");
                        audio_.reattach(name);
                    }
                }
                if let Some(sink) = sink {
                    let Ok(node) = registry_.bind::<Node, _>(global) else {
                        log::warn!("cannot bind {name}");
                        return;
                    };
                    log::info!("sink {name} is node {}", global.id);
                    let mut inner = audio_.inner.borrow_mut();
                    set_volume(&node, sink, inner.mix);
                    inner.nodes.push(Bound {
                        sink,
                        id: global.id,
                        serial: props
                            .get("object.serial")
                            .and_then(|serial| serial.parse().ok())
                            .unwrap_or_default(),
                        node,
                    });
                    drop(inner);
                    audio_.take_default();
                    audio_.route_running();
                }
            })
            .global_remove(move |id| {
                let gone = {
                    let mut inner = audio__.inner.borrow_mut();
                    let gone = inner.candidates.remove(&id).map(|candidate| candidate.name);
                    gone.filter(|name| inner.follow && inner.target.as_ref() == Some(name))
                };
                if let Some(name) = gone {
                    audio__.drop_upstream(&name);
                }
                let mut inner = audio__.inner.borrow_mut();
                inner.nodes.retain(|bound| bound.id != id);
                inner.streams.retain(|stream| stream.id != id);
                inner.names.remove(&id);
                inner.links.remove(&id);
            })
            .register()
    }

    /// Listens on the default metadata for a mixer moving the loopbacks' streams.
    fn watch(
        &self,
        metadata: std::result::Result<Metadata, pipewire::Error>,
    ) {
        let Ok(metadata) = metadata else {
            log::warn!("cannot bind the default metadata: moves in a mixer are not followed");
            return;
        };
        let audio = self.clone();
        let listener = metadata
            .add_listener_local()
            .property(move |subject, key, _type, value| {
                if key == Some("target.object") {
                    audio.moved(subject, value);
                }
                0
            })
            .register();
        self.inner.borrow_mut().metadata = Some((metadata, listener));
    }

    /// Once all three sinks are bound, and the upstream was chosen from the old default, sets
    /// GLOBAL as the default sink with `wpctl set-default`, if the config asks. Not undone on
    /// stop: WirePlumber falls back to the next sink by priority.
    fn take_default(&self) {
        let id = {
            let mut inner = self.inner.borrow_mut();
            let all_bound = Sink::ALL
                .iter()
                .all(|sink| inner.nodes.iter().any(|bound| bound.sink == *sink));
            if inner.taken || !inner.take_default || !all_bound || inner.target.is_none() {
                return;
            }
            inner.taken = true;
            inner
                .nodes
                .iter()
                .rfind(|bound| bound.sink == Sink::Global)
                .map(|bound| bound.id)
        };
        let Some(id) = id else {
            return;
        };
        match Command::new("wpctl")
            .args(["set-default", &id.to_string()])
            .status()
        {
            Ok(status) if status.success() => log::info!("default sink is now GLOBAL (node {id})"),
            Ok(status) => log::warn!("wpctl set-default {id}: {status}"),
            Err(err) => log::warn!("wpctl set-default {id}: {err}"),
        }
    }

    /// Once all three sinks are bound, moves every playback stream a rule matches, as
    /// `route <stream> <sink>` does; the WirePlumber script only sees streams as they appear.
    /// Streams that named a sink themselves are left alone, as the script leaves them.
    fn route_running(&self) {
        let moves: Vec<(u32, String, Sink)> = {
            let mut inner = self.inner.borrow_mut();
            let all_bound = Sink::ALL
                .iter()
                .all(|sink| inner.nodes.iter().any(|bound| bound.sink == *sink));
            if inner.swept || !all_bound {
                return;
            }
            inner.swept = true;
            inner
                .streams
                .iter()
                .filter(|stream| {
                    !stream.props.contains_key("target.object")
                        && !stream.props.contains_key("node.target")
                })
                .filter_map(|stream| {
                    let sink = route::matching(&inner.rules, &stream.props)?;
                    Some((stream.id, stream.name.clone(), sink))
                })
                .collect()
        };
        let mut audio = self.clone();
        let mut moved = 0;
        for (id, name, sink) in moves {
            match audio.move_stream(id, sink) {
                Ok(()) => {
                    log::info!("routed running stream {name} (node {id}) to {sink}");
                    moved += 1;
                }
                Err(err) => log::warn!("cannot route running stream {name}: {err}"),
            }
        }
        log::info!("routed {moved} running streams");
    }

    /// The upstream is gone (unplugged): ends the loopbacks, which played to nothing, and
    /// picks again by the same rule. Streams on our sinks are moved by WirePlumber meanwhile.
    fn drop_upstream(
        &self,
        name: &str,
    ) {
        log::info!("upstream {name} is gone; picking another");
        {
            let mut inner = self.inner.borrow_mut();
            for mut child in inner.loopbacks.drain(..) {
                let _ = child.kill();
                let _ = child.wait();
            }
            inner.target = None;
            inner.swept = false;
        }
        // The default may be the user's new choice, else the best by priority
        let default = default_node_name().ok();
        let (chosen, how) = {
            let inner = self.inner.borrow();
            match default.filter(|default| inner.candidates.values().any(|c| &c.name == default)) {
                Some(name) => (Some(name), "the default sink"),
                None => (best(&inner.candidates), "highest priority real sink"),
            }
        };
        // The remembered sink stays what it was: the stand-in does not replace it
        match chosen {
            Some(name) => {
                log::info!("upstream is {name} ({how})");
                {
                    let mut inner = self.inner.borrow_mut();
                    inner.how = format!("{how}, stand-in");
                    inner.standin = true;
                }
                self.spawn(name);
            }
            None => log::warn!("no real sink left; waiting for one"),
        }
    }

    fn spawn(
        &self,
        target: String,
    ) {
        let mut inner = self.inner.borrow_mut();
        let sinks = inner.sinks.clone();
        log::info!(
            "routing {}, {} and {} to {target}",
            sinks.game,
            sinks.chat,
            sinks.global
        );
        for sink in Sink::ALL {
            let name = sinks.name(sink);
            let child = Command::new("pw-loopback")
                .args([
                    "-n",
                    name,
                    &format!("--capture-props=media.class=Audio/Sink node.description={name}"),
                    "-P",
                    &target,
                ])
                .spawn();
            match child {
                Ok(child) => inner.loopbacks.push(child),
                Err(err) => log::error!("pw-loopback for {name}: {err}"),
            }
        }
        inner.target = Some(target);
    }

    /// Removes the sinks.
    pub fn close(&self) {
        let mut inner = self.inner.borrow_mut();
        inner.nodes.clear();
        for mut child in inner.loopbacks.drain(..) {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
impl Audio for PwAudio {
    fn set_mix(
        &mut self,
        mix: Mix,
    ) {
        let mut inner = self.inner.borrow_mut();
        inner.mix = mix;
        // GLOBAL is not on the dial: it keeps the full volume it was given
        for bound in inner
            .nodes
            .iter()
            .filter(|bound| bound.sink != Sink::Global)
        {
            set_volume(&bound.node, bound.sink, mix);
        }
    }

    fn sinks(&self) -> Vec<(Sink, u32, String)> {
        let inner = self.inner.borrow();
        inner
            .nodes
            .iter()
            .map(|bound| (bound.sink, bound.id, inner.sinks.node_name(bound.sink)))
            .collect()
    }

    fn streams(&self) -> Vec<(Stream, HashMap<String, String>)> {
        let inner = self.inner.borrow();
        inner
            .streams
            .iter()
            .map(|stream| {
                let target = inner
                    .links
                    .values()
                    .find(|(output, _input)| *output == stream.id)
                    .and_then(|(_output, input)| inner.names.get(input).cloned());
                let info = Stream {
                    id: stream.id,
                    name: stream.name.clone(),
                    binary: stream.binary.clone(),
                    target,
                    rule: None,
                };
                (info, stream.props.clone())
            })
            .collect()
    }

    fn output(&self) -> Output {
        let inner = self.inner.borrow();
        Output {
            name: inner.target.clone(),
            how: inner.how.clone(),
        }
    }

    /// Refuses our own sinks and anything that is not a real sink now present.
    fn set_output(
        &mut self,
        wanted: &str,
    ) -> Result<()> {
        let name = {
            let inner = self.inner.borrow();
            let by_id = wanted.parse::<u32>().ok().and_then(|id| {
                inner
                    .candidates
                    .get(&id)
                    .map(|candidate| candidate.name.clone())
                    .or_else(|| inner.names.get(&id).cloned())
            });
            by_id.unwrap_or_else(|| wanted.to_string())
        };
        {
            let inner = self.inner.borrow();
            if inner.sinks.sink_of(&name).is_some() || inner.sinks.is_output(&name) {
                return Err(anyhow!("{name} is one of novamix's own sinks"));
            }
            if !inner.candidates.values().any(|c| c.name == name) {
                return Err(anyhow!("{name} is not a real sink that is present"));
            }
            if inner.target.as_deref() == Some(&name) {
                drop(inner);
                self.remember(&name);
                self.inner.borrow_mut().how = "output command".into();
                return Ok(());
            }
        }
        self.retarget(&name, "output command")?;
        self.remember(&name);
        Ok(())
    }

    /// Sets the stream's `target.object` on the default metadata, as the WirePlumber script
    /// does; WirePlumber links it again.
    fn move_stream(
        &mut self,
        id: u32,
        sink: Sink,
    ) -> Result<()> {
        let inner = self.inner.borrow();
        if !inner.streams.iter().any(|stream| stream.id == id) {
            return Err(anyhow!("No playback stream with node id {id}"));
        }
        let serial = inner
            .nodes
            .iter()
            // The newest: a leftover sink of the same name may still be listed
            .rfind(|bound| bound.sink == sink)
            .ok_or(anyhow!("The {sink} sink does not exist"))?
            .serial;
        let status = Command::new("pw-metadata")
            .args([
                &id.to_string(),
                "target.object",
                &serial.to_string(),
                "Spa:Id",
            ])
            .status()?;
        if !status.success() {
            return Err(anyhow!("pw-metadata: {status}"));
        }
        Ok(())
    }
}

/// The default sink's `node.name`, from `wpctl inspect @DEFAULT_AUDIO_SINK@`.
fn default_node_name() -> Result<String> {
    let out = Command::new("wpctl")
        .args(["inspect", "@DEFAULT_AUDIO_SINK@"])
        .output()?;
    if !out.status.success() {
        return Err(anyhow!(
            "wpctl inspect @DEFAULT_AUDIO_SINK@: {}",
            out.status
        ));
    }
    inspect_name(&String::from_utf8_lossy(&out.stdout))
}

/// The highest-priority candidate: `priority.session`, then `priority.driver`, then the name.
fn best(candidates: &HashMap<u32, Candidate>) -> Option<String> {
    candidates
        .values()
        .max_by(|a, b| a.priority.cmp(&b.priority).then(b.name.cmp(&a.name)))
        .map(|candidate| candidate.name.clone())
}

/// The default sink at start; never one of ours.
fn default_sink(sinks: &Names) -> Result<String> {
    let name = default_node_name()?;
    refuse_own(name, sinks)
}

/// Says once at start when the default sink is one of ours; novamix never changes the default.
fn log_default(sinks: &Names) {
    let Ok(name) = default_node_name() else {
        return;
    };
    match sinks.sink_of(&name) {
        Some(Sink::Global) => log::info!("default sink is {name}: apps on it keep full volume"),
        Some(_) => log::info!(
            "default sink is {name}: apps on System Default are on the dial; \
             `wpctl set-default` picks another"
        ),
        None => {}
    }
}

/// Reads `node.name` out of `wpctl inspect`.
fn inspect_name(inspect: &str) -> Result<String> {
    inspect
        .lines()
        .find_map(|line| {
            let (key, value) = line.trim_start_matches([' ', '*']).split_once(" = ")?;
            (key == "node.name").then(|| value.trim_matches('"').to_string())
        })
        .ok_or(anyhow!("The default sink has no node.name"))
}

/// Refuses our own sinks, which a daemon that did not stop cleanly may have left as the default.
fn refuse_own(
    name: String,
    sinks: &Names,
) -> Result<String> {
    if sinks.sink_of(&name).is_some() {
        return Err(anyhow!(
            "The default sink is {name}, novamix's own (left over, or a race)"
        ));
    }
    Ok(name)
}

/// Sets the node's channel volumes, as pavucontrol or `pactl set-sink-volume` would.
fn set_volume(
    node: &Node,
    sink: Sink,
    mix: Mix,
) {
    let level = match sink {
        Sink::Game => mix.game,
        Sink::Chat => mix.chat,
        Sink::Global => 100,
    };
    let props = Value::Object(Object {
        type_: SpaTypes::ObjectParamProps.as_raw(),
        id: ParamType::Props.as_raw(),
        properties: vec![Property {
            key: pipewire::spa::sys::SPA_PROP_channelVolumes,
            flags: PropertyFlags::empty(),
            value: Value::ValueArray(ValueArray::Float(vec![gain(level); CHANNELS])),
        }],
    });
    let pod_buf = match PodSerializer::serialize(std::io::Cursor::new(Vec::new()), &props) {
        Ok((cursor, _len)) => cursor.into_inner(),
        Err(err) => {
            log::error!("volume pod: {err:?}");
            return;
        }
    };
    let Some(pod) = Pod::from_bytes(&pod_buf) else {
        return;
    };
    node.set_param(ParamType::Props, 0, pod);
}

#[cfg(test)]
mod test {
    use super::*;

    /// `wpctl inspect @DEFAULT_AUDIO_SINK@` on a desktop, cut short.
    const INSPECT: &str = r#"id 57, type PipeWire:Interface:Node
    audio.channels = "2"
  * media.class = "Audio/Sink"
  * node.description = "Family 17h/19h HD Audio Controller Analog Stereo"
  * node.name = "alsa_output.pci-0000_0c_00.4.analog-stereo"
    object.serial = "60"
"#;

    #[test]
    fn test_best() {
        let candidate = |name: &str, session, driver| Candidate {
            serial: 0,
            name: name.to_string(),
            priority: (session, driver),
        };
        let mut candidates = HashMap::new();
        assert!(best(&candidates).is_none());
        candidates.insert(1, candidate("speakers", 1000, 1000));
        candidates.insert(2, candidate("headset", 2000, 500));
        candidates.insert(3, candidate("hdmi", 2000, 900));
        assert!(best(&candidates).unwrap() == "hdmi");
    }

    #[test]
    fn test_node_name() {
        assert!(inspect_name(INSPECT).unwrap() == "alsa_output.pci-0000_0c_00.4.analog-stereo");
        assert!(inspect_name("id 57, type PipeWire:Interface:Node\n").is_err());
        let own = INSPECT.replace("alsa_output.pci-0000_0c_00.4.analog-stereo", "input.GAME");
        let name = inspect_name(&own).unwrap();
        assert!(refuse_own(name, &Names::default()).is_err());
    }
}
