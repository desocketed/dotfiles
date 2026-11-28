//! Workspace state from `niri msg --json event-stream`.
//!
//! The 8 keys are Niri's 8 named workspaces Q W E R A S D F (the names in
//! modules/wm/niri/workspaces.kdl), in key order. Unnamed workspaces and other
//! names are ignored.

use std::collections::BTreeMap;
use std::collections::HashMap;

use serde::Deserialize;

use crate::protocol::Slot;
use crate::protocol::WORKSPACE_COUNT;
use crate::protocol::WorkspaceState;
use crate::width::Span;

pub const NAMES: [&str; WORKSPACE_COUNT] = ["Q", "W", "E", "R", "A", "S", "D", "F"];

#[derive(Deserialize, Debug)]
pub struct Workspace {
    pub id: u64,
    pub name: Option<String>,
    pub output: Option<String>,
    pub is_urgent: bool,
    pub is_active: bool,
    pub is_focused: bool,
}

#[derive(Deserialize, Debug)]
pub struct Window {
    pub id: u64,
    pub workspace_id: Option<u64>,
    #[serde(default)]
    pub is_focused: bool,
    pub layout: Option<Layout>,
}

/// A window's place in the layout (niri-ipc `WindowLayout`).
#[derive(Deserialize, Debug, Clone, Copy)]
pub struct Layout {
    /// (column, tile in column), 1-based, for tiled windows.
    pub pos_in_scrolling_layout: Option<(usize, usize)>,
    /// Size of the tile, borders included.
    pub tile_size: (f64, f64),
}

/// A screencast (niri-ipc `Cast`); one that is paused does not count as sharing.
#[derive(Deserialize, Debug)]
pub struct Cast {
    pub stream_id: u64,
    pub is_active: bool,
}

#[derive(Deserialize, Debug)]
pub enum Event {
    CastsChanged { casts: Vec<Cast> },
    CastStartedOrChanged { cast: Cast },
    CastStopped { stream_id: u64 },
    WorkspacesChanged { workspaces: Vec<Workspace> },
    WorkspaceActivated { id: u64, focused: bool },
    WorkspaceUrgencyChanged { id: u64, urgent: bool },
    WindowsChanged { windows: Vec<Window> },
    WindowOpenedOrChanged { window: Window },
    WindowClosed { id: u64 },
    WindowFocusChanged { id: Option<u64> },
    WindowLayoutsChanged { changes: Vec<(u64, Layout)> },
}

/// Logical width of each output, from `niri msg --json outputs`.
pub fn parse_outputs(json: &str) -> HashMap<String, f64> {
    #[derive(Deserialize)]
    struct Output {
        logical: Option<Logical>,
    }
    #[derive(Deserialize)]
    struct Logical {
        width: f64,
    }
    let outputs: HashMap<String, Output> = serde_json::from_str(json).unwrap_or_default();
    outputs
        .into_iter()
        .filter_map(|(name, output)| Some((name, output.logical?.width)))
        .collect()
}

/// None for events we do not track (serde's `other` only covers unit variants).
pub fn parse(line: &str) -> Option<Event> {
    serde_json::from_str(line).ok()
}

#[derive(Default)]
pub struct Tracker {
    workspaces: HashMap<u64, Workspace>,
    windows: HashMap<u64, Option<u64>>,
    layouts: HashMap<u64, Layout>,
    focused: Option<u64>,
    focused_window: Option<u64>,
    output_widths: HashMap<String, f64>,
    /// Whether each screencast, by stream ID, is streaming frames.
    casts: HashMap<u64, bool>,
}

impl Tracker {
    /// Whether a screencast is streaming frames.
    pub fn casting(&self) -> bool {
        self.casts.values().any(|&active| active)
    }

    pub fn set_output_widths(
        &mut self,
        widths: HashMap<String, f64>,
    ) {
        self.output_widths = widths;
    }

    pub fn apply(
        &mut self,
        event: Event,
    ) {
        match event {
            Event::CastsChanged { casts } => {
                self.casts = casts.into_iter().map(|cast| (cast.stream_id, cast.is_active)).collect();
            }
            Event::CastStartedOrChanged { cast } => {
                self.casts.insert(cast.stream_id, cast.is_active);
            }
            Event::CastStopped { stream_id } => {
                self.casts.remove(&stream_id);
            }
            Event::WorkspacesChanged { workspaces } => {
                self.focused = workspaces.iter().find(|ws| ws.is_focused).map(|ws| ws.id);
                self.workspaces = workspaces.into_iter().map(|ws| (ws.id, ws)).collect();
            }
            Event::WorkspaceActivated { id, focused } => {
                let output = self.workspaces.get(&id).map(|ws| ws.output.clone());
                if let Some(output) = output {
                    for ws in self
                        .workspaces
                        .values_mut()
                        .filter(|ws| ws.output == output)
                    {
                        ws.is_active = ws.id == id;
                    }
                }
                if focused {
                    self.focused = Some(id);
                }
            }
            Event::WorkspaceUrgencyChanged { id, urgent } => {
                if let Some(ws) = self.workspaces.get_mut(&id) {
                    ws.is_urgent = urgent;
                }
            }
            Event::WindowsChanged { windows } => {
                self.windows.clear();
                self.layouts.clear();
                for window in windows {
                    self.add_window(window);
                }
            }
            Event::WindowOpenedOrChanged { window } => self.add_window(window),
            Event::WindowClosed { id } => {
                self.windows.remove(&id);
                self.layouts.remove(&id);
                if self.focused_window == Some(id) {
                    self.focused_window = None;
                }
            }
            Event::WindowFocusChanged { id } => self.focused_window = id,
            Event::WindowLayoutsChanged { changes } => {
                for (id, layout) in changes {
                    self.layouts.insert(id, layout);
                }
            }
        }
    }

    fn add_window(
        &mut self,
        window: Window,
    ) {
        self.windows.insert(window.id, window.workspace_id);
        match window.layout {
            Some(layout) => self.layouts.insert(window.id, layout),
            None => self.layouts.remove(&window.id),
        };
        if window.is_focused {
            self.focused_window = Some(window.id);
        }
    }

    /// The output holding the focused workspace.
    pub fn focused_output(&self) -> Option<&str> {
        self.workspaces.get(&self.focused?)?.output.as_deref()
    }

    /// Logical width of the focused output, once it has been queried.
    pub fn focused_width(&self) -> Option<f64> {
        self.output_widths.get(self.focused_output()?).copied()
    }

    /// Left edge and width of each scrolling column of a workspace, in layout
    /// coordinates (no gaps in the config), by column index.
    fn columns(
        &self,
        workspace: u64,
    ) -> BTreeMap<usize, (f64, f64)> {
        let mut widths = BTreeMap::new();
        for (id, _) in self
            .windows
            .iter()
            .filter(|(_, ws)| **ws == Some(workspace))
        {
            if let Some(layout) = self.layouts.get(id)
                && let Some((column, _)) = layout.pos_in_scrolling_layout
            {
                let width = widths.entry(column).or_insert(0.0f64);
                *width = width.max(layout.tile_size.0);
            }
        }
        let mut left = 0.0;
        widths
            .into_iter()
            .map(|(column, width)| {
                let place = (column, (left, width));
                left += width;
                place
            })
            .collect()
    }

    /// The focused workspace's whole scrolling strip: one span per column,
    /// the focused window's column marked, and the strip's length in logical
    /// pixels, at least the output's width so a short strip is not stretched.
    /// Floating windows are not part of the strip. None when there is no
    /// focused workspace or its output has no known width.
    pub fn strip(&self) -> Option<(Vec<Span>, f64)> {
        let focused = self.focused?;
        let width = self.focused_width()?;
        let columns = self.columns(focused);
        let focused_column = self
            .focused_window
            .and_then(|id| self.layouts.get(&id)?.pos_in_scrolling_layout)
            .map(|(column, _)| column);
        let total: f64 = columns.values().map(|(_, w)| w).sum();
        let spans = columns
            .iter()
            .map(|(column, (left, w))| Span {
                left: *left,
                right: left + w,
                focused: focused_column == Some(*column),
            })
            .collect();
        Some((spans, total.max(width)))
    }

    pub fn states(&self) -> [Slot; WORKSPACE_COUNT] {
        let focused_output = self
            .focused
            .and_then(|id| self.workspaces.get(&id))
            .map(|ws| &ws.output);
        let mut states = [Slot {
            state: WorkspaceState::Empty,
            dim: false,
        }; WORKSPACE_COUNT];
        for ws in self.workspaces.values() {
            let Some(name) = &ws.name else {
                continue;
            };
            let Some(key) = NAMES.iter().position(|n| n.eq_ignore_ascii_case(name)) else {
                continue;
            };
            let state = if ws.is_urgent {
                WorkspaceState::Urgent
            } else if self.focused == Some(ws.id) {
                WorkspaceState::Focused
            } else if ws.is_active {
                WorkspaceState::Visible
            } else if self.windows.values().any(|w| *w == Some(ws.id)) {
                WorkspaceState::Occupied
            } else {
                WorkspaceState::Empty
            };
            let dim = focused_output.is_some_and(|output| *output != ws.output);
            states[key] = Slot { state, dim };
        }
        states
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn replay(stream: &str) -> Tracker {
        let mut tracker = Tracker::default();
        for line in stream.lines() {
            if let Some(event) = parse(line) {
                tracker.apply(event);
            }
        }
        tracker
    }

    #[test]
    fn test_casts() {
        let mut tracker = Tracker::default();
        let casts =
            |json: &str| parse(&format!(r#"{{"CastsChanged":{{"casts":{json}}}}}"#)).unwrap();
        let cast = |active| {
            format!(
                r#"[{{"stream_id":1,"session_id":1,"kind":"PipeWire","target":{{"Output":{{"name":"DP-1"}}}},"is_dynamic_target":false,"is_active":{active},"pid":null,"pw_node_id":42}}]"#
            )
        };
        tracker.apply(casts(&cast(true)));
        assert!(tracker.casting());
        tracker.apply(casts(&cast(false)));
        assert!(!tracker.casting());
        tracker.apply(casts(&cast(true)));
        tracker.apply(casts("[]"));
        assert!(!tracker.casting());
        // Started and stopped one at a time, as niri sends them after the first list
        let one = cast(true);
        let one = one.trim_start_matches('[').trim_end_matches(']');
        tracker.apply(parse(&format!(r#"{{"CastStartedOrChanged":{{"cast":{one}}}}}"#)).unwrap());
        assert!(tracker.casting());
        tracker.apply(parse(r#"{"CastStopped":{"stream_id":1}}"#).unwrap());
        assert!(!tracker.casting());
    }

    #[test]
    fn test_width_fixture() {
        let mut tracker = Tracker::default();
        tracker.set_output_widths(parse_outputs(include_str!(
            "../tests/fixtures/outputs.json"
        )));
        for line in include_str!("../tests/fixtures/event-stream.jsonl").lines() {
            if let Some(event) = parse(line) {
                tracker.apply(event);
            }
        }
        // The strip of workspace Q on the 2560 wide DP-1: column 1 (window 10,
        // focused) is 860 wide, column 2 (window 14) 1720 wide; the floating
        // window 16 is not part of it; window 11 is on another workspace.
        let (spans, length) = tracker.strip().unwrap();
        let span = |left, right, focused| Span {
            left,
            right,
            focused,
        };
        assert_eq!(length, 2580.0);
        assert_eq!(spans, [span(0.0, 860.0, true), span(860.0, 2580.0, false)]);
        assert_eq!(
            crate::width::levels(&spans, length),
            [255, 255, 26, 26, 26, 26]
        );
        tracker.apply(Event::WindowFocusChanged { id: None });
        let (spans, _) = tracker.strip().unwrap();
        assert!(spans.iter().all(|span| !span.focused));
    }

    #[test]
    fn test_event_stream_fixture() {
        use WorkspaceState::*;
        let slot = |state, dim| Slot { state, dim };
        let tracker = replay(include_str!("../tests/fixtures/event-stream.jsonl"));
        // Q focused (has a window), W occupied, E urgent, R empty,
        // A visible (active on DP-2, window closed), S..F absent; the unnamed workspace is ignored.
        // Everything on DP-2 (unfocused display) is dimmed.
        assert_eq!(
            tracker.states(),
            [
                slot(Focused, false),
                slot(Occupied, false),
                slot(Urgent, false),
                slot(Empty, false),
                slot(Visible, true),
                slot(Empty, false),
                slot(Empty, false),
                slot(Empty, false),
            ]
        );
    }

    // Real windows of a workspace 1 (2560 px wide): the strip is 1700 +
    // 1274 = 2974 px, longer than the output, and each key is a sixth of it.
    #[test]
    fn test_strip_of_real_windows() {
        let stream = [
            r#"{"WorkspacesChanged":{"workspaces":[{"id":1,"idx":1,"name":"Q","output":"DP-1","is_urgent":false,"is_active":true,"is_focused":true,"active_window_id":215}]}}"#,
            r#"{"WindowsChanged":{"windows":[{"id":215,"workspace_id":1,"is_focused":true,"is_floating":false,"layout":{"pos_in_scrolling_layout":[1,1],"tile_size":[1700.0,1376.0],"window_size":[1692,1368],"tile_pos_in_workspace_view":null,"window_offset_in_tile":[4.0,4.0]}},{"id":237,"workspace_id":1,"is_focused":false,"is_floating":false,"layout":{"pos_in_scrolling_layout":[2,1],"tile_size":[1274.0,1376.0],"window_size":[1266,1368],"tile_pos_in_workspace_view":null,"window_offset_in_tile":[4.0,4.0]}}]}}"#,
        ];
        let mut tracker = Tracker::default();
        tracker.set_output_widths(parse_outputs(include_str!(
            "../tests/fixtures/outputs.json"
        )));
        for line in stream {
            tracker.apply(parse(line).unwrap());
        }
        let levels = |tracker: &Tracker| {
            let (spans, length) = tracker.strip().unwrap();
            assert_eq!(length, 2974.0);
            crate::width::levels(&spans, length)
        };
        // Sixths of 2974 are 495.67 px: 215 (0..1700) fills three and a bit (the
        // fourth is shared with 237, the brighter wins), 237 shows dim after.
        assert_eq!(levels(&tracker), [255, 255, 255, 110, 26, 25]);
        // Focus moves to 237: the shares swap, nothing scrolls.
        tracker.apply(Event::WindowFocusChanged { id: Some(237) });
        assert_eq!(levels(&tracker), [26, 26, 25, 145, 255, 255]);
    }

    // A strip shorter than the output is not stretched over it.
    #[test]
    fn test_short_strip_is_not_stretched() {
        let stream = [
            r#"{"WorkspacesChanged":{"workspaces":[{"id":1,"idx":1,"name":"Q","output":"DP-1","is_urgent":false,"is_active":true,"is_focused":true,"active_window_id":5}]}}"#,
            r#"{"WindowsChanged":{"windows":[{"id":5,"workspace_id":1,"is_focused":true,"is_floating":false,"layout":{"pos_in_scrolling_layout":[1,1],"tile_size":[1280.0,1376.0],"window_size":[1272,1368],"tile_pos_in_workspace_view":null,"window_offset_in_tile":[4.0,4.0]}}]}}"#,
        ];
        let mut tracker = Tracker::default();
        tracker.set_output_widths(parse_outputs(include_str!(
            "../tests/fixtures/outputs.json"
        )));
        for line in stream {
            tracker.apply(parse(line).unwrap());
        }
        let (spans, length) = tracker.strip().unwrap();
        assert_eq!(length, 2560.0);
        assert_eq!(
            crate::width::levels(&spans, length),
            [255, 255, 255, 0, 0, 0]
        );
    }
}
