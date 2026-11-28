//! The look of the LEDs: `~/.config/moonlander-leds/config.toml`, with command
//! line flags on top and values `set` over the control socket on top of both.
//! The file is re-read while running, so edits apply live.

use std::path::PathBuf;
use std::time::SystemTime;

use clap::Args;
use clap::ValueEnum;
use serde::Deserialize;

use crate::protocol::Palette;
use crate::protocol::Rgb;

/// Colours are `#rrggbb`; `dim` is the brightness of an unfocused display.
#[derive(Args, Clone, Copy, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Look {
    /// Colour of a workspace without windows (default: off).
    #[arg(long, value_name = "RGB")]
    empty: Option<RgbText>,
    /// Colour of a workspace with windows.
    #[arg(long, value_name = "RGB")]
    occupied: Option<RgbText>,
    /// Colour of the focused workspace.
    #[arg(long, value_name = "RGB")]
    focused: Option<RgbText>,
    /// Colour of a workspace with an urgent window.
    #[arg(long, value_name = "RGB")]
    urgent: Option<RgbText>,
    /// Colour of the active workspace of an unfocused display.
    #[arg(long, value_name = "RGB")]
    visible: Option<RgbText>,
    /// Brightness of workspaces on an unfocused display, 0.0 to 1.0.
    #[arg(long, value_name = "FACTOR")]
    dim: Option<f32>,
    /// Colour of the play/pause key while playing.
    #[arg(long, value_name = "RGB")]
    media_playing: Option<RgbText>,
    /// Colour of the play/pause key while paused or stopped.
    #[arg(long, value_name = "RGB")]
    media_paused: Option<RgbText>,
    /// Colour of the play/pause key with no player.
    #[arg(long, value_name = "RGB")]
    media_none: Option<RgbText>,
    /// Colour of the previous/next key when the player can go that way.
    #[arg(long, value_name = "RGB")]
    media_skip: Option<RgbText>,
    /// Colour of the previous/next key when it cannot.
    #[arg(long, value_name = "RGB")]
    media_noskip: Option<RgbText>,
}

#[derive(Clone, Copy)]
struct RgbText(Rgb);

impl std::str::FromStr for RgbText {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, String> {
        text.parse().map(Self)
    }
}

impl<'de> Deserialize<'de> for RgbText {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

impl Look {
    /// `self` wins over `base`.
    fn over(
        self,
        base: Look,
    ) -> Look {
        Look {
            empty: self.empty.or(base.empty),
            occupied: self.occupied.or(base.occupied),
            focused: self.focused.or(base.focused),
            urgent: self.urgent.or(base.urgent),
            visible: self.visible.or(base.visible),
            dim: self.dim.or(base.dim),
            media_playing: self.media_playing.or(base.media_playing),
            media_paused: self.media_paused.or(base.media_paused),
            media_none: self.media_none.or(base.media_none),
            media_skip: self.media_skip.or(base.media_skip),
            media_noskip: self.media_noskip.or(base.media_noskip),
        }
    }

    fn palette(&self) -> Palette {
        let base = Palette::default();
        let pick = |text: Option<RgbText>, default| text.map_or(default, |text| text.0);
        Palette {
            empty: pick(self.empty, base.empty),
            occupied: pick(self.occupied, base.occupied),
            focused: pick(self.focused, base.focused),
            urgent: pick(self.urgent, base.urgent),
            visible: pick(self.visible, base.visible),
            dim: self.dim.unwrap_or(base.dim),
            media_playing: pick(self.media_playing, base.media_playing),
            media_paused: pick(self.media_paused, base.media_paused),
            media_none: pick(self.media_none, base.media_none),
            media_skip: pick(self.media_skip, base.media_skip),
            media_noskip: pick(self.media_noskip, base.media_noskip),
        }
    }
}

/// A palette entry, named as in config.toml.
#[derive(ValueEnum, Clone, Copy, Debug, Eq, PartialEq)]
#[value(rename_all = "snake_case")]
pub enum Key {
    Empty,
    Occupied,
    Focused,
    Urgent,
    Visible,
    Dim,
    MediaPlaying,
    MediaPaused,
    MediaNone,
    MediaSkip,
    MediaNoskip,
}

impl Key {
    pub fn name(self) -> String {
        self.to_possible_value().unwrap().get_name().to_string()
    }

    /// The entry's value in `palette`, as `#rrggbb` or the dim factor.
    pub fn value(
        self,
        palette: &Palette,
    ) -> String {
        let color = match self {
            Self::Dim => return format!("{}", palette.dim),
            Self::Empty => palette.empty,
            Self::Occupied => palette.occupied,
            Self::Focused => palette.focused,
            Self::Urgent => palette.urgent,
            Self::Visible => palette.visible,
            Self::MediaPlaying => palette.media_playing,
            Self::MediaPaused => palette.media_paused,
            Self::MediaNone => palette.media_none,
            Self::MediaSkip => palette.media_skip,
            Self::MediaNoskip => palette.media_noskip,
        };
        format!("{color}")
    }
}

/// The palette from the config file, the flags and `set`; follows file changes.
pub struct Config {
    path: Option<PathBuf>,
    flags: Look,
    session: Look,
    seen: Option<SystemTime>,
    file: Look,
}

impl Config {
    pub fn new(flags: Look) -> Self {
        let path = dirs::config_dir().map(|dir| dir.join("moonlander-leds/config.toml"));
        Self {
            path,
            flags,
            session: Look::default(),
            seen: None,
            file: Look::default(),
        }
    }

    /// The current palette. A missing file means defaults; a broken one keeps
    /// the last good values and logs.
    pub fn palette(&mut self) -> Palette {
        let modified = self
            .path
            .as_ref()
            .and_then(|path| path.metadata().ok())
            .and_then(|meta| meta.modified().ok());
        if modified != self.seen {
            self.seen = modified;
            match self.read() {
                Ok(look) => self.file = look,
                Err(err) => log::warn!("config: {err}"),
            }
        }
        self.session.over(self.flags.over(self.file)).palette()
    }

    /// Override one entry until the daemon restarts; checks the value the
    /// same way config.toml does.
    pub fn set(
        &mut self,
        key: Key,
        value: &str,
    ) -> Result<(), String> {
        let line = match key {
            Key::Dim => format!("dim = {value}"),
            _ => format!("{} = {:?}", key.name(), value),
        };
        let look: Look = toml::from_str(&line).map_err(|err| err.message().to_string())?;
        if look.dim.is_some_and(|dim| !(0.0..=1.0).contains(&dim)) {
            return Err(format!("dim must be 0.0 to 1.0, got {value}"));
        }
        self.session = look.over(self.session);
        Ok(())
    }

    pub fn reset(&mut self) {
        self.session = Look::default();
    }

    fn read(&self) -> Result<Look, String> {
        let Some(path) = &self.path else {
            return Ok(Look::default());
        };
        match std::fs::read_to_string(path) {
            Ok(text) => toml::from_str(&text).map_err(|err| format!("{}: {err}", path.display())),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Look::default()),
            Err(err) => Err(format!("{}: {err}", path.display())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_overrides_and_reset() {
        let mut config = Config {
            path: None,
            flags: Look::default(),
            session: Look::default(),
            seen: None,
            file: Look::default(),
        };
        config.set(Key::Focused, "#00ff88").unwrap();
        config.set(Key::Dim, "0.25").unwrap();
        let palette = config.palette();
        assert_eq!(Key::Focused.value(&palette), "#00ff88");
        assert_eq!(Key::Dim.value(&palette), "0.25");
        assert!(config.set(Key::Urgent, "red").is_err());
        assert!(config.set(Key::Dim, "2").is_err());
        config.reset();
        assert_eq!(config.palette(), Palette::default());
    }
}
