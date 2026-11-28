//! A window over the running daemon's control socket: the workspace and media
//! keys as the keyboard shows them, and a colour picker per palette entry.
//! Picks apply live and last until the daemon restarts, like `set`.

use std::io::Read;
use std::io::Write;
use std::os::unix::net::UnixStream;
use std::time::Duration;

use gpui::App;
use gpui::Application;
use gpui::Bounds;
use gpui::ClickEvent;
use gpui::Context;
use gpui::Div;
use gpui::ElementId;
use gpui::Stateful;
use gpui::Window;
use gpui::WindowBounds;
use gpui::WindowOptions;
use gpui::div;
use gpui::prelude::*;
use gpui::px;
use gpui::rgb;
use gpui::size;

const REFRESH: Duration = Duration::from_secs(1);

const PRESETS: [&str; 16] = [
    "#000000", "#303030", "#808080", "#ffffff", "#ff0000", "#ff8000", "#ffff00", "#80ff00",
    "#00ff00", "#00ff7f", "#00ffff", "#0080ff", "#0000ff", "#8000ff", "#ff00ff", "#ff0080",
];

const DIMS: [&str; 5] = ["0", "0.25", "0.5", "0.75", "1"];

/// One request line to the daemon, its reply text back.
fn request(line: &str) -> Result<String, String> {
    let path = dirs::runtime_dir()
        .ok_or("No XDG_RUNTIME_DIR")?
        .join("moonlander-leds.sock");
    let mut stream = UnixStream::connect(&path)
        .map_err(|err| format!("Daemon not reachable at {}: {err}", path.display()))?;
    writeln!(stream, "{line}").map_err(|err| err.to_string())?;
    let _ = stream.shutdown(std::net::Shutdown::Write);
    let mut reply = String::new();
    stream
        .read_to_string(&mut reply)
        .map_err(|err| err.to_string())?;
    match reply.strip_prefix("error: ") {
        Some(err) => Err(err.trim_end().to_string()),
        None => Ok(reply),
    }
}

fn color(hex: &str) -> u32 {
    u32::from_str_radix(hex.trim_start_matches('#'), 16).unwrap_or(0)
}

/// What the daemon reported last: `get` (entry, value) pairs and `status`.
#[derive(Default)]
struct Leds {
    entries: Vec<(String, String)>,
    status: Vec<(String, String)>,
    error: Option<String>,
}

impl Leds {
    fn reload(&mut self) {
        let pairs = |text: String, separator: &str| {
            text.lines()
                .filter_map(|line| line.split_once(separator))
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect()
        };
        match request("get").and_then(|get| Ok((get, request("status")?))) {
            Ok((get, status)) => {
                self.entries = pairs(get, " ");
                self.status = pairs(status, ": ");
                self.error = None;
            }
            Err(err) => self.error = Some(err),
        }
    }

    fn send(
        &mut self,
        line: &str,
    ) {
        match request(line) {
            Ok(_) => self.reload(),
            Err(err) => self.error = Some(err),
        }
    }

    fn entry(
        &self,
        name: &str,
    ) -> u32 {
        self.entries
            .iter()
            .find(|(key, _)| key == name)
            .map_or(0, |(_, value)| color(value))
    }

    fn status(
        &self,
        name: &str,
    ) -> &str {
        self.status
            .iter()
            .find(|(key, _)| key == name)
            .map_or("unknown", |(_, value)| value)
    }

    /// The workspace keys Q..F with their palette entry, from `status`.
    fn workspace_keys(&self) -> Vec<(String, u32)> {
        self.status("workspaces")
            .split(", ")
            .filter_map(|slot| {
                let mut words = slot.split(' ');
                let name = words.next()?.to_uppercase();
                let state = words.next()?;
                Some((name, self.entry(state)))
            })
            .collect()
    }

    /// Previous, play/pause and next with their palette entry, from `status`.
    fn media_keys(&self) -> Vec<(String, u32)> {
        let media = self.status("media");
        let skip = |way| match media.contains(&format!("{way} yes")) {
            true => self.entry("media_skip"),
            false => self.entry("media_noskip"),
        };
        let play = if media.starts_with("playing") {
            self.entry("media_playing")
        } else if media.starts_with("paused") {
            self.entry("media_paused")
        } else {
            self.entry("media_none")
        };
        vec![
            ("Prev".to_string(), skip("previous")),
            ("Play".to_string(), play),
            ("Next".to_string(), skip("next")),
        ]
    }
}

struct LedsView {
    leds: Leds,
}

impl LedsView {
    fn new(cx: &mut Context<Self>) -> Self {
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(REFRESH).await;
                let updated = this.update(cx, |view, cx| {
                    view.leds.reload();
                    cx.notify();
                });
                if updated.is_err() {
                    return;
                }
            }
        })
        .detach();
        let mut leds = Leds::default();
        leds.reload();
        Self { leds }
    }

    fn button(
        &self,
        id: String,
        line: String,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        div()
            .id(ElementId::Name(id.into()))
            .cursor_pointer()
            .on_click(cx.listener(move |view, _: &ClickEvent, _window, cx| {
                view.leds.send(&line);
                cx.notify();
            }))
    }
}

fn keycap(
    label: String,
    background: u32,
) -> Div {
    div()
        .w(px(56.0))
        .h(px(56.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .border_1()
        .border_color(rgb(0x606060))
        .bg(rgb(background))
        .child(div().px_1().rounded_sm().bg(rgb(0x202020)).child(label))
}

impl Render for LedsView {
    fn render(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let workspaces = div().flex().gap_2().children(
            self.leds
                .workspace_keys()
                .into_iter()
                .map(|(name, background)| keycap(name, background)),
        );
        let media = div().flex().gap_2().children(
            self.leds
                .media_keys()
                .into_iter()
                .map(|(name, background)| keycap(name, background)),
        );
        let mut pickers = div().flex().flex_col().gap_1();
        for (key, value) in self.leds.entries.clone() {
            let choices = if key == "dim" {
                &DIMS[..]
            } else {
                &PRESETS[..]
            };
            let mut row = div()
                .flex()
                .items_center()
                .gap_1()
                .child(div().w(px(120.0)).child(key.clone()))
                .child(div().w(px(72.0)).child(value.clone()));
            for choice in choices {
                let line = format!("set {key} {choice}");
                let chosen = *choice == value;
                let swatch = self
                    .button(format!("{key}={choice}"), line, cx)
                    .h(px(22.0))
                    .rounded_sm()
                    .border_1()
                    .border_color(rgb(if chosen { 0xffffff } else { 0x404040 }));
                row = row.child(match key.as_str() {
                    "dim" => swatch.px_2().child(choice.to_string()),
                    _ => swatch.w(px(22.0)).bg(rgb(color(choice))),
                });
            }
            pickers = pickers.child(row);
        }
        let reset = self
            .button("reset".to_string(), "reset".to_string(), cx)
            .px_2()
            .rounded_sm()
            .border_1()
            .border_color(rgb(0x808080))
            .child("Reset to config");
        div()
            .size_full()
            .p_4()
            .flex()
            .flex_col()
            .gap_3()
            .bg(rgb(0x181818))
            .text_color(rgb(0xe0e0e0))
            .text_sm()
            .child(format!("keyboard: {}", self.leds.status("keyboard")))
            .children(
                self.leds
                    .error
                    .clone()
                    .map(|err| div().text_color(rgb(0xff6060)).child(err)),
            )
            .child("Workspace keys")
            .child(workspaces)
            .child("Media keys")
            .child(media)
            .child("Colours (until the daemon restarts)")
            .child(pickers)
            .child(div().flex().child(reset))
    }
}

fn main() {
    env_logger::init();
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(760.0), px(620.0)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            app_id: Some("moonlander-leds-gui".to_string()),
            ..Default::default()
        };
        cx.open_window(options, |_window, cx| cx.new(LedsView::new))
            .unwrap();
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
    });
}
