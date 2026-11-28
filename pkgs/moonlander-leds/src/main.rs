//! Drives the Moonlander's LEDs from desktop state over raw HID.
//!
//! Each state source (Niri, MPRIS) runs on its own thread and sends `Update`s
//! to the main loop, which keeps the latest `State` and writes a report for
//! every part that changed, and all of it again after a (re)connect.

mod balance;
mod config;
mod control;
mod labels;
mod layout;
mod lock;
mod media;
mod niri;
mod output;
mod protocol;
mod usage;
mod width;

use std::collections::HashMap;
use std::io::BufRead;
use std::io::BufReader;
use std::process::Command;
use std::process::Stdio;
use std::time::Duration;
use std::time::Instant;

use clap::Parser;
use clap::ValueEnum;
use crossbeam_channel::Receiver;
use crossbeam_channel::RecvTimeoutError;
use crossbeam_channel::Sender;
use log::debug;
use log::info;
use log::warn;

use crate::config::Config;
use crate::config::Key;
use crate::config::Look;
use crate::control::Reply;
use crate::control::Request;
use crate::output::Output;
use crate::protocol::Media;
use crate::protocol::Palette;
use crate::protocol::Report;
use crate::protocol::Slot;
use crate::protocol::WIDTH_KEY_COUNT;
use crate::protocol::WORKSPACE_COUNT;
use crate::protocol::balance_report;
use crate::protocol::balance_set;
use crate::protocol::cast_report;
use crate::protocol::lock_report;
use crate::protocol::media_keys_report;
use crate::protocol::palette_report;
use crate::protocol::width_report;

pub const RETRY: Duration = Duration::from_secs(2);
/// How often the keyboard's key counters are read.
const USAGE_POLL: Duration = Duration::from_secs(300);
/// How often the loop looks for packets from the keyboard.
const POLL: Duration = Duration::from_millis(50);

/// Without a command, run the daemon; the commands talk to a running one.
#[derive(Parser)]
#[command(name = "moonlander-leds", args_conflicts_with_subcommands = true)]
struct Args {
    #[command(subcommand)]
    request: Option<Request>,
    /// Print reports as hex lines instead of writing to the keyboard. Without
    /// --stdin this follows the live Niri stream (run with RUST_LOG=info or
    /// debug for what it sees).
    #[arg(long)]
    print: bool,
    /// `niri msg --json outputs` output to use when replaying stdin.
    #[arg(long, value_name = "FILE")]
    outputs: Option<std::path::PathBuf>,
    /// Read the Niri event stream (JSON lines) from stdin, then exit at EOF.
    #[arg(long)]
    stdin: bool,
    #[command(flatten)]
    look: Look,
}

/// One source's latest state. Add a variant per new source.
pub enum Update {
    /// Whether the Niri event stream is up. Without it the keyboard is left
    /// alone, so its keymap falls back to its plain lighting.
    Niri(bool),
    Workspaces([Slot; WORKSPACE_COUNT]),
    Media(Media),
    /// Brightness of the 1-6 keys, none while no window is focused.
    Width(Option<[u8; WIDTH_KEY_COUNT]>),
    /// ChatMix balance, -100 (game) to 100 (chat); none while novamix is unreachable.
    Balance(Option<i32>),
    /// Whether a screencast is running.
    Casting(bool),
    /// Whether the locker is running.
    Locked(bool),
    /// A control socket request and where its reply goes.
    Request(Request, Sender<Reply>),
}

#[derive(Default)]
struct State {
    niri: bool,
    workspaces: Option<[Slot; WORKSPACE_COUNT]>,
    media: Option<Media>,
    width: Option<Option<[u8; WIDTH_KEY_COUNT]>>,
    balance: Option<i32>,
    casting: bool,
    locked: bool,
}

impl State {
    fn apply(
        &mut self,
        update: Update,
    ) {
        match update {
            Update::Niri(up) => {
                self.niri = up;
                if !up {
                    self.workspaces = None;
                    self.width = None;
                    self.casting = false;
                }
            }
            Update::Workspaces(states) => self.workspaces = Some(states),
            Update::Media(media) => self.media = Some(media),
            Update::Width(levels) => self.width = Some(levels),
            Update::Balance(balance) => self.balance = balance,
            Update::Casting(casting) => self.casting = casting,
            Update::Locked(locked) => self.locked = locked,
            Update::Request(..) => unreachable!(),
        }
    }

    fn status(
        &self,
        connected: bool,
    ) -> String {
        let keyboard = if connected {
            "connected"
        } else {
            "not connected"
        };
        let mut text = format!("keyboard: {keyboard}\n");
        match &self.workspaces {
            Some(slots) => {
                let names: Vec<String> = slots
                    .iter()
                    .zip(niri::NAMES)
                    .map(|(slot, name)| {
                        let dim = if slot.dim { " (dim)" } else { "" };
                        format!("{name} {:?}{dim}", slot.state).to_lowercase()
                    })
                    .collect();
                text += &format!("workspaces: {}\n", names.join(", "));
            }
            None => text += "workspaces: unknown\n",
        }
        match &self.media {
            Some(media) => {
                let yes = |can| if can { "yes" } else { "no" };
                text += &format!(
                    "media: {:?}, previous {}, next {}\n",
                    media.playback,
                    yes(media.can_previous),
                    yes(media.can_next)
                )
                .to_lowercase();
            }
            None => text += "media: unknown\n",
        }
        match self.width {
            Some(Some(levels)) => text += &format!("width keys: {levels:?}\n"),
            Some(None) => text += "width keys: no focused window\n",
            None => text += "width keys: unknown\n",
        }
        text += &format!("screencast: {}\n", if self.casting { "yes" } else { "no" });
        text += &format!("locked: {}\n", if self.locked { "yes" } else { "no" });
        match self.balance {
            Some(balance) => text += &format!("balance: {balance}\n"),
            None => text += "balance: unknown\n",
        }
        text
    }

    fn reports(
        &self,
        palette: &Palette,
    ) -> Vec<Report> {
        // The keymap drops every host overlay once reports stop coming.
        if !self.niri {
            return Vec::new();
        }
        let mut reports = vec![palette_report(palette)];
        reports.extend(self.workspaces.iter().map(protocol::workspaces_report));
        reports.extend(
            self.media
                .iter()
                .map(|media| media_keys_report(media, palette)),
        );
        // No tiled window focused: all four keys off.
        reports.extend(
            self.width
                .map(|levels| width_report(&levels.unwrap_or_default())),
        );
        reports.extend(self.balance.map(balance_report));
        reports.push(cast_report(self.casting));
        reports.push(lock_report(self.locked));
        reports
    }
}

fn follow_niri(
    lines: impl BufRead,
    tracker: &mut niri::Tracker,
    outputs: impl Fn() -> Option<HashMap<String, f64>>,
    tx: &Sender<Update>,
) {
    let _ = tx.send(Update::Niri(true));
    let mut last_focus = (None, None);
    for line in lines.lines() {
        let Ok(line) = line else {
            return;
        };
        let Some(event) = niri::parse(&line) else {
            debug!("skipping event: {line}");
            continue;
        };
        debug!("event: {event:?}");
        let is_workspaces = matches!(event, niri::Event::WorkspacesChanged { .. });
        tracker.apply(event);
        // Only a query tells an output's size: ask again whenever the focused
        // output has none, and on workspace changes (outputs come and go).
        if tracker.focused_output().is_some()
            && (tracker.focused_width().is_none() || is_workspaces)
            && let Some(widths) = outputs()
        {
            tracker.set_output_widths(widths);
        }
        let focus = (
            tracker.focused_output().map(str::to_owned),
            tracker.focused_width(),
        );
        if focus != last_focus {
            match &focus {
                (Some(name), Some(width)) => info!("focused output {name}, {width} px wide"),
                (Some(name), None) => warn!("focused output {name} has no known width"),
                _ => info!("no focused output"),
            }
            last_focus = focus;
        }
        let _ = tx.send(Update::Casting(tracker.casting()));
        let _ = tx.send(Update::Workspaces(tracker.states()));
        let levels = tracker
            .strip()
            .map(|(spans, width)| width::levels(&spans, width));
        debug!("width keys: {levels:?}");
        let _ = tx.send(Update::Width(levels));
    }
}

fn query_outputs() -> Option<HashMap<String, f64>> {
    let output = Command::new("niri")
        .args(["msg", "--json", "outputs"])
        .output()
        .inspect_err(|err| warn!("cannot query niri outputs: {err}"))
        .ok()?;
    let widths = niri::parse_outputs(&String::from_utf8_lossy(&output.stdout));
    if widths.is_empty() {
        warn!(
            "niri outputs gave no sizes ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Some(widths)
}

fn niri_source(tx: Sender<Update>) {
    loop {
        let child = Command::new("niri")
            .args(["msg", "--json", "event-stream"])
            .stdout(Stdio::piped())
            .spawn();
        match child {
            Ok(mut child) => {
                let stdout = child.stdout.take().unwrap();
                follow_niri(
                    BufReader::new(stdout),
                    &mut niri::Tracker::default(),
                    query_outputs,
                    &tx,
                );
                let _ = child.wait();
            }
            Err(err) => warn!("cannot run niri: {err}"),
        }
        let _ = tx.send(Update::Niri(false));
        std::thread::sleep(RETRY);
    }
}

fn run(
    mut output: Output,
    rx: Receiver<Update>,
    mut config: Config,
    sets: Sender<i32>,
) {
    let mut state = State::default();
    let mut sent: Vec<Report> = Vec::new();
    let mut sent_at = Instant::now();
    let mut usage_db = None;
    let mut last_usage = Instant::now();
    let mut next_chunk = None;
    loop {
        let mut resend = false;
        match rx.recv_timeout(POLL) {
            Ok(Update::Request(request, tx_reply)) => {
                let reply = answer(request, &state, &mut config, output.is_connected());
                let _ = tx_reply.send(reply);
            }
            Ok(update) => state.apply(update),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        if output.is_connected() && last_usage.elapsed() >= USAGE_POLL {
            last_usage = Instant::now();
            next_chunk = Some((usage::Table::Keys, 0));
        }
        // One chunk per turn, so the replies are read before the next request.
        if let Some((table, chunk)) = next_chunk.filter(|_| output.is_connected()) {
            output.send(&usage::request_report(table, chunk));
            next_chunk = table.next(chunk);
        }
        while let Some(packet) = output.read() {
            if let Some((table, counts)) = usage::decode(&packet) {
                if usage_db.is_none() {
                    usage_db = usage::open_default()
                        .map_err(|err| warn!("cannot open key usage database: {err}"))
                        .ok();
                }
                if let Some(Err(err)) = usage_db
                    .as_ref()
                    .map(|db| usage::add(db, usage::hour_now(), table, &counts))
                {
                    warn!("cannot save key usage: {err}");
                }
            }
            if let Some(balance) = balance_set(&packet) {
                info!("keyboard sets the balance to {balance}");
                let _ = sets.send(balance);
            }
        }
        // Idle: refresh the keyboard anyway, in case it lost a report or
        // rebooted without the write failing.
        resend |= sent_at.elapsed() >= RETRY;
        // A fresh connection (or an idle tick) resends everything.
        if !output.is_connected() && output.connect() {
            resend = true;
        }
        let reports = state.reports(&config.palette());
        // The keymap fades the width keys out when reports stop coming, so
        // resend before that even when events keep the loop busy.
        if resend || reports != sent {
            for report in &reports {
                output.send(report);
            }
            sent = reports;
            sent_at = Instant::now();
        }
    }
}

fn answer(
    request: Request,
    state: &State,
    config: &mut Config,
    connected: bool,
) -> Reply {
    match request {
        Request::Set { key, value } => {
            config.set(key, &value)?;
            Ok(String::new())
        }
        Request::Get { key: Some(key) } => Ok(format!("{}\n", key.value(&config.palette()))),
        Request::Get { key: None } => {
            let palette = config.palette();
            let lines: Vec<String> = Key::value_variants()
                .iter()
                .map(|key| format!("{} {}\n", key.name(), key.value(&palette)))
                .collect();
            Ok(lines.concat())
        }
        Request::Reset => {
            config.reset();
            Ok(String::new())
        }
        Request::Status => Ok(state.status(connected)),
        Request::Usage { .. } => Err("usage reads the key-usage file itself".to_string()),
    }
}

/// `<command> help` means `help <command>`.
fn help_last(args: impl Iterator<Item = String>) -> Vec<String> {
    let mut args: Vec<String> = args.collect();
    if args.len() == 3 && args[2] == "help" {
        args.swap(1, 2);
    }
    args
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let args = Args::parse_from(help_last(std::env::args()));
    if let Some(Request::Usage { days }) = args.request {
        let now = usage::hour_now();
        let since = now.saturating_sub(days.unwrap_or(30) * 24);
        let buckets = usage::load(&usage::open_default()?, since)?;
        print!("{}", usage::render(&buckets, since));
        return Ok(());
    }
    if let Some(request) = &args.request {
        print!("{}", control::send(request)?);
        return Ok(());
    }
    let output = if args.print {
        Output::Print
    } else {
        Output::hid()?
    };
    let config = Config::new(args.look);
    let (tx, rx) = crossbeam_channel::unbounded();
    if args.stdin {
        // The stream is finite: replay it, then let the main loop drain.
        let stdin = std::io::stdin();
        let widths = args
            .outputs
            .and_then(|path| std::fs::read_to_string(path).ok())
            .map(|json| niri::parse_outputs(&json));
        follow_niri(
            stdin.lock(),
            &mut niri::Tracker::default(),
            || widths.clone(),
            &tx,
        );
        drop(tx);
        run(output, rx, config, crossbeam_channel::unbounded().0);
    } else {
        // A debugging run beside the service must not take over its socket.
        if !args.print {
            let control_tx = tx.clone();
            std::thread::spawn(move || control::serve(control_tx));
        }
        let media_tx = tx.clone();
        std::thread::spawn(move || media::source(media_tx));
        let (sets_tx, sets_rx) = crossbeam_channel::unbounded();
        let lock_tx = tx.clone();
        std::thread::spawn(move || lock::source(lock_tx));
        let balance_tx = tx.clone();
        std::thread::spawn(move || balance::source(sets_rx, balance_tx));
        std::thread::spawn(move || niri_source(tx));
        run(output, rx, config, sets_tx);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_reports_without_niri() {
        let palette = Palette::default();
        let mut state = State::default();
        state.apply(Update::Media(Media::default()));
        assert!(state.reports(&palette).is_empty());

        state.apply(Update::Niri(true));
        state.apply(Update::Width(Some([255; WIDTH_KEY_COUNT])));
        assert_eq!(state.reports(&palette).len(), 5);

        // Niri goes away: nothing is sent, and its state is forgotten.
        state.apply(Update::Niri(false));
        assert!(state.reports(&palette).is_empty());
        state.apply(Update::Niri(true));
        assert_eq!(state.reports(&palette).len(), 4);
    }

    #[test]
    fn test_follows_the_focused_output() {
        // Two real-shaped outputs; the focus moves from DP-1 (2560 wide) to
        // DP-2 (1920 wide), where one tiled window fills the left half.
        let workspaces = r#"{"WorkspacesChanged":{"workspaces":[{"id":1,"idx":1,"name":"Q","output":"DP-1","is_urgent":false,"is_active":true,"is_focused":true,"active_window_id":null},{"id":2,"idx":1,"name":"W","output":"DP-2","is_urgent":false,"is_active":true,"is_focused":false,"active_window_id":7}]}}"#;
        let window = r#"{"WindowsChanged":{"windows":[{"id":7,"title":"t","app_id":"a","pid":1,"workspace_id":2,"is_focused":false,"is_floating":false,"is_urgent":false,"layout":{"pos_in_scrolling_layout":[1,1],"tile_size":[960.0,1080.0],"window_size":[960,1080],"tile_pos_in_workspace_view":null,"window_offset_in_tile":[0.0,0.0]}}]}}"#;
        let focus = r#"{"WorkspaceActivated":{"id":2,"focused":true}}
{"WindowFocusChanged":{"id":7}}"#;
        let stream = format!("{workspaces}\n{window}\n{focus}\n");
        let outputs = niri::parse_outputs(include_str!("../tests/fixtures/outputs.json"));
        let (tx, rx) = crossbeam_channel::unbounded();
        let queries = std::cell::Cell::new(0);
        follow_niri(
            stream.as_bytes(),
            &mut niri::Tracker::default(),
            || {
                queries.set(queries.get() + 1);
                Some(outputs.clone())
            },
            &tx,
        );
        drop(tx);
        let widths: Vec<_> = rx
            .iter()
            .filter_map(|update| match update {
                Update::Width(levels) => Some(levels),
                _ => None,
            })
            .collect();
        // On DP-1 nothing is open; on DP-2 the window covers its left half.
        assert_eq!(widths[0], Some([0; WIDTH_KEY_COUNT]));
        assert_eq!(widths.last(), Some(&Some([255, 255, 255, 0, 0, 0])));
        // Sizes were queried once, with the workspaces, and not again.
        assert_eq!(queries.get(), 1);
    }
}
