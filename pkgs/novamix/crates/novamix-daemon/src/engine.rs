//! What the daemon does with each event, apart from the devices so it runs on fakes in tests.

use std::collections::HashMap;
use std::time::Duration;
use std::time::Instant;

use novamix::Result;
use novamix::ipc::Output;
use novamix::ipc::Request;
use novamix::ipc::Response;
use novamix::ipc::Routes;
use novamix::ipc::Rule;
use novamix::ipc::Status;
use novamix::ipc::Stream;
use novamix::mix::Mix;
use novamix::mix::Sink;
use novamix::protocol::Report;

use crate::route;

/// Something that holds the two sinks' volumes: PipeWire, or a fake in tests.
pub trait Audio {
    fn set_mix(
        &mut self,
        mix: Mix,
    );

    /// The sinks that exist, with their node ids.
    fn sinks(&self) -> Vec<(Sink, u32, String)>;

    /// The playback streams alive now, each with all its PipeWire properties.
    fn streams(&self) -> Vec<(Stream, HashMap<String, String>)>;

    fn move_stream(
        &mut self,
        id: u32,
        sink: Sink,
    ) -> Result<()>;

    /// The real sink the sinks play to, and how it was chosen.
    fn output(&self) -> Output;

    /// Plays the sinks to the real sink `wanted` (a `node.name` or node id) and remembers it.
    fn set_output(
        &mut self,
        wanted: &str,
    ) -> Result<()>;
}

/// What reaches the main loop from the device thread and the control socket.
pub enum Event {
    Connected,
    Disconnected,
    Report(Report),
    Request(Request, crossbeam_channel::Sender<Response>),
}

/// How long after writing a mix to the base station a report of that same mix counts as its
/// echo rather than the dial.
const ECHO_WINDOW: Duration = Duration::from_millis(300);

pub struct Engine<A: Audio> {
    pub audio: A,
    connected: bool,
    mix: Mix,
    rules: Vec<Rule>,
    /// Mixes set from the socket go to the device thread, to move the base station's dial.
    tx_dial: Option<crossbeam_channel::Sender<Mix>>,
    /// The last mix written to the base station, and when.
    written: Option<(Mix, Instant)>,
}
impl<A: Audio> Engine<A> {
    /// Starts at the dial's centre: both sinks at full volume.
    pub fn new(mut audio: A) -> Self {
        let mix = Mix::full();
        audio.set_mix(mix);
        Self {
            audio,
            connected: false,
            mix,
            rules: Vec::new(),
            tx_dial: None,
            written: None,
        }
    }

    /// Writes mixes set from the socket to the base station.
    pub fn with_dial(
        mut self,
        tx_dial: crossbeam_channel::Sender<Mix>,
    ) -> Self {
        self.tx_dial = Some(tx_dial);
        self
    }

    /// The routing rules `Request::Routes` reports.
    pub fn with_rules(
        mut self,
        rules: Vec<Rule>,
    ) -> Self {
        self.rules = rules;
        self
    }

    pub fn handle(
        &mut self,
        event: Event,
    ) {
        match event {
            Event::Connected => {
                log::info!("base station connected");
                self.connected = true;
            }
            // The sinks keep their last volumes until the dial is back
            Event::Disconnected => {
                log::info!("base station disconnected");
                self.connected = false;
            }
            Event::Report(Report::ChatMix(mix)) => {
                if let Some((written, at)) = self.written.take()
                    && written == mix
                    && at.elapsed() < ECHO_WINDOW
                {
                    log::debug!("echo of {mix}");
                    return;
                }
                self.set(mix);
            }
            Event::Report(Report::Volume(_) | Report::Other(_)) => {}
            Event::Request(request, tx_reply) => {
                let response = self.request(request);
                let _ = tx_reply.send(response);
            }
        }
    }

    fn request(
        &mut self,
        request: Request,
    ) -> Response {
        match request {
            Request::Status => Response::Status(self.status()),
            Request::Set(mix) => {
                let mix = Mix::new(mix.game, mix.chat);
                self.set(mix);
                if self.connected
                    && let Some(tx_dial) = &self.tx_dial
                    && tx_dial.send(mix).is_ok()
                {
                    self.written = Some((mix, Instant::now()));
                }
                Response::Ok
            }
            Request::Routes => Response::Routes(Routes {
                rules: self.rules.clone(),
                streams: self
                    .audio
                    .streams()
                    .into_iter()
                    .map(|(stream, props)| Stream {
                        rule: route::matching(&self.rules, &props),
                        ..stream
                    })
                    .collect(),
            }),
            Request::Move(id, sink) => match self.audio.move_stream(id, sink) {
                Ok(()) => Response::Ok,
                Err(err) => Response::Error(err.to_string()),
            },
            Request::Output(wanted) => {
                if let Some(wanted) = wanted
                    && let Err(err) = self.audio.set_output(&wanted)
                {
                    return Response::Error(err.to_string());
                }
                Response::Output(self.audio.output())
            }
        }
    }

    fn status(&self) -> Status {
        Status {
            connected: self.connected,
            sinks: self.audio.sinks(),
            mix: self.mix,
        }
    }

    fn set(
        &mut self,
        mix: Mix,
    ) {
        if mix == self.mix {
            return;
        }
        log::debug!("{mix}");
        self.mix = mix;
        self.audio.set_mix(mix);
    }
}

#[cfg(test)]
mod test {
    use novamix::protocol::decode;

    use super::*;

    /// Records every mix it is given.
    #[derive(Default)]
    struct FakeAudio {
        mixes: Vec<Mix>,
        streams: Vec<Stream>,
        output: Option<String>,
    }
    impl Audio for FakeAudio {
        fn set_mix(
            &mut self,
            mix: Mix,
        ) {
            self.mixes.push(mix);
        }

        fn sinks(&self) -> Vec<(Sink, u32, String)> {
            vec![
                (Sink::Game, 41, "input.GAME".into()),
                (Sink::Chat, 42, "input.CHAT".into()),
                (Sink::Global, 43, "input.GLOBAL".into()),
            ]
        }

        fn streams(&self) -> Vec<(Stream, HashMap<String, String>)> {
            self.streams
                .iter()
                .map(|stream| {
                    let props = [
                        ("application.name".to_string(), stream.name.clone()),
                        ("media.role".to_string(), "Communication".to_string()),
                    ];
                    (stream.clone(), props.into_iter().collect())
                })
                .collect()
        }

        fn output(&self) -> Output {
            Output {
                name: self.output.clone(),
                how: "fake".into(),
            }
        }

        fn set_output(
            &mut self,
            wanted: &str,
        ) -> Result<()> {
            if wanted.starts_with("input.") {
                return Err(anyhow::anyhow!("{wanted} is one of novamix's own sinks"));
            }
            self.output = Some(wanted.to_string());
            Ok(())
        }

        fn move_stream(
            &mut self,
            id: u32,
            sink: Sink,
        ) -> Result<()> {
            let stream = self
                .streams
                .iter_mut()
                .find(|stream| stream.id == id)
                .ok_or(anyhow::anyhow!("No playback stream with node id {id}"))?;
            stream.target = Some(format!("input.{}", sink.to_string().to_uppercase()));
            Ok(())
        }
    }

    fn report(bytes: &[u8]) -> Event {
        Event::Report(decode(bytes).unwrap())
    }

    fn ask(
        engine: &mut Engine<FakeAudio>,
        request: Request,
    ) -> Response {
        let (tx_reply, rx_reply) = crossbeam_channel::bounded(1);
        engine.handle(Event::Request(request, tx_reply));
        rx_reply.recv().unwrap()
    }

    #[test]
    fn test_dial_sets_sinks() {
        let mut engine = Engine::new(FakeAudio::default());
        // Turning the dial towards chat, as the base station reports it
        engine.handle(report(&[0x07, 0x45, 100, 100]));
        engine.handle(report(&[0x07, 0x45, 80, 100]));
        engine.handle(report(&[0x07, 0x45, 80, 100]));
        engine.handle(report(&[0x07, 0x25, 10]));
        engine.handle(report(&[0x07, 0x45, 0, 100]));
        assert!(engine.audio.mixes == [Mix::full(), Mix::new(80, 100), Mix::new(0, 100)]);
    }

    #[test]
    fn test_status_follows_device() {
        let mut engine = Engine::new(FakeAudio::default());
        engine.handle(Event::Connected);
        engine.handle(report(&[0x07, 0x45, 100, 30]));
        let expected = Status {
            connected: true,
            sinks: vec![
                (Sink::Game, 41, "input.GAME".into()),
                (Sink::Chat, 42, "input.CHAT".into()),
                (Sink::Global, 43, "input.GLOBAL".into()),
            ],
            mix: Mix::new(100, 30),
        };
        assert!(ask(&mut engine, Request::Status) == Response::Status(expected));
        // Unplugged: the volumes stay where the dial left them
        engine.handle(Event::Disconnected);
        let Response::Status(status) = ask(&mut engine, Request::Status) else {
            unreachable!()
        };
        assert!(!status.connected);
        assert!(engine.audio.mixes.last() == Some(&Mix::new(100, 30)));
    }

    #[test]
    fn test_set_request() {
        let mut engine = Engine::new(FakeAudio::default());
        assert!(
            ask(
                &mut engine,
                Request::Set(Mix {
                    game: 50,
                    chat: 250
                })
            ) == Response::Ok
        );
        assert!(engine.audio.mixes.last() == Some(&Mix::new(50, 100)));
    }

    #[test]
    fn test_set_writes_dial() {
        let (tx_dial, rx_dial) = crossbeam_channel::unbounded();
        let mut engine = Engine::new(FakeAudio::default()).with_dial(tx_dial);
        // Nothing to write to while the base station is away
        ask(&mut engine, Request::Set(Mix::new(10, 100)));
        assert!(rx_dial.try_recv().is_err());
        engine.handle(Event::Connected);
        ask(&mut engine, Request::Set(Mix::new(50, 100)));
        assert!(rx_dial.try_recv() == Ok(Mix::new(50, 100)));
    }

    #[test]
    fn test_echo_ignored() {
        let (tx_dial, _rx_dial) = crossbeam_channel::unbounded();
        let mut engine = Engine::new(FakeAudio::default()).with_dial(tx_dial);
        engine.handle(Event::Connected);
        ask(&mut engine, Request::Set(Mix::new(50, 100)));
        // The base station echoes the write, then the user turns the dial
        engine.handle(report(&[0x07, 0x45, 50, 100]));
        assert!(engine.written.is_none());
        engine.handle(report(&[0x07, 0x45, 60, 100]));
        assert!(engine.audio.mixes == [Mix::full(), Mix::new(50, 100), Mix::new(60, 100)]);
        // A stale write is no echo: the dial's same mix still applies
        ask(&mut engine, Request::Set(Mix::new(30, 100)));
        engine.written = Some((Mix::new(30, 100), Instant::now() - ECHO_WINDOW));
        engine.handle(report(&[0x07, 0x45, 60, 100]));
        assert!(engine.audio.mixes.last() == Some(&Mix::new(60, 100)));
    }

    #[test]
    fn test_routes_and_move() {
        let rule = Rule {
            sink: Sink::Chat,
            matches: vec![vec![("media.role".into(), "Communication".into())]],
        };
        let audio = FakeAudio {
            streams: vec![Stream {
                id: 90,
                name: "Firefox".into(),
                binary: "firefox".into(),
                target: Some("headset".into()),
                rule: None,
            }],
            ..FakeAudio::default()
        };
        let mut engine = Engine::new(audio).with_rules(vec![rule.clone()]);
        assert!(ask(&mut engine, Request::Move(90, Sink::Chat)) == Response::Ok);
        assert!(ask(&mut engine, Request::Move(90, Sink::Global)) == Response::Ok);
        assert!(ask(&mut engine, Request::Move(90, Sink::Chat)) == Response::Ok);
        assert!(matches!(
            ask(&mut engine, Request::Move(91, Sink::Chat)),
            Response::Error(_)
        ));
        let Response::Routes(routes) = ask(&mut engine, Request::Routes) else {
            unreachable!()
        };
        assert!(routes.rules == [rule]);
        assert!(routes.streams[0].target.as_deref() == Some("input.CHAT"));
        // The fake gives every stream the Communication role, which the rule matches
        assert!(routes.streams[0].rule == Some(Sink::Chat));
        let mut engine = Engine::new(FakeAudio {
            streams: routes.streams,
            ..FakeAudio::default()
        });
        let Response::Routes(routes) = ask(&mut engine, Request::Routes) else {
            unreachable!()
        };
        assert!(routes.streams[0].rule.is_none());
    }

    #[test]
    fn test_output() {
        let mut engine = Engine::new(FakeAudio::default());
        assert!(
            ask(&mut engine, Request::Output(None))
                == Response::Output(Output {
                    name: None,
                    how: "fake".into()
                })
        );
        let Response::Output(output) = ask(&mut engine, Request::Output(Some("speakers".into())))
        else {
            panic!("no output");
        };
        assert!(output.name.as_deref() == Some("speakers"));
        assert!(matches!(
            ask(&mut engine, Request::Output(Some("input.GAME".into()))),
            Response::Error(_)
        ));
    }
}
