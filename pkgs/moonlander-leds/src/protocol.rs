//! Raw HID packet layout, mirrored by `protocol.h` in the QMK keymap
//! (the keymap). The first byte of a packet is the event; the rest is
//! padded with zeros to the 32-byte raw HID report.

pub const REPORT_LEN: usize = 32;
pub const WORKSPACE_COUNT: usize = 8;

pub const EVENT_WORKSPACES: u8 = 4;
pub const EVENT_PALETTE: u8 = 5;
pub const EVENT_MEDIA_KEYS: u8 = 6;
pub const EVENT_WIDTH: u8 = 7;
pub const EVENT_BALANCE: u8 = 8;
pub const EVENT_BALANCE_SET: u8 = 9;
pub const EVENT_CAST: u8 = 10;
pub const EVENT_LOCK: u8 = 11;

/// The 1-5 keys and the right half's 6 key.
pub const WIDTH_KEY_COUNT: usize = 6;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum WorkspaceState {
    Empty = 0,
    Occupied = 1,
    Focused = 2,
    Urgent = 3,
    Visible = 4,
}

/// High bit of a slot byte: the workspace is on a display that is not focused.
pub const DIM_FLAG: u8 = 0x80;

/// One key's workspace: its state, and whether its display is unfocused.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Slot {
    pub state: WorkspaceState,
    pub dim: bool,
}

pub type Report = [u8; REPORT_LEN];

fn slot_byte(slot: Slot) -> u8 {
    slot.state as u8 | if slot.dim { DIM_FLAG } else { 0 }
}

pub fn workspaces_report(slots: &[Slot; WORKSPACE_COUNT]) -> Report {
    let mut report = [0; REPORT_LEN];
    report[0] = EVENT_WORKSPACES;
    for (byte, slot) in report[1..].iter_mut().zip(slots) {
        *byte = slot_byte(*slot);
    }
    report
}

/// An LED colour.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Rgb(pub [u8; 3]);

impl std::str::FromStr for Rgb {
    type Err = String;

    /// Parses `#rrggbb` (the `#` is optional).
    fn from_str(text: &str) -> Result<Self, String> {
        let hex = text.strip_prefix('#').unwrap_or(text);
        let value = (hex.len() == 6)
            .then(|| u32::from_str_radix(hex, 16).ok())
            .flatten()
            .ok_or_else(|| format!("expected a colour like #rrggbb, got {text:?}"))?;
        Ok(Self([(value >> 16) as u8, (value >> 8) as u8, value as u8]))
    }
}

impl std::fmt::Display for Rgb {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        let [r, g, b] = self.0;
        write!(f, "#{r:02x}{g:02x}{b:02x}")
    }
}

/// How each workspace state looks, and how much an unfocused display dims.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Palette {
    pub empty: Rgb,
    pub occupied: Rgb,
    pub focused: Rgb,
    pub urgent: Rgb,
    pub visible: Rgb,
    /// Brightness multiplier of an unfocused display, 0.0 to 1.0.
    pub dim: f32,
    /// Play/pause key while playing, paused, and with no player.
    pub media_playing: Rgb,
    pub media_paused: Rgb,
    pub media_none: Rgb,
    /// Previous/next key when the player can, and cannot, go that way.
    pub media_skip: Rgb,
    pub media_noskip: Rgb,
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            empty: Rgb([0, 0, 0]),
            occupied: Rgb([0, 0, 0xff]),
            focused: Rgb([0xff, 0xff, 0xff]),
            urgent: Rgb([0xff, 0, 0]),
            visible: Rgb([0x30, 0x30, 0x30]),
            dim: 0.5,
            media_playing: Rgb([0, 0xff, 0]),
            media_paused: Rgb([0xff, 0, 0]),
            media_none: Rgb([0x30, 0x30, 0x30]),
            media_skip: Rgb([0, 0xff, 0x7f]),
            media_noskip: Rgb([0, 0, 0]),
        }
    }
}

/// Colours in `WorkspaceState` order, then the dim factor as 0..=255.
pub fn palette_report(palette: &Palette) -> Report {
    let mut report = [0; REPORT_LEN];
    report[0] = EVENT_PALETTE;
    let colors = [
        palette.empty,
        palette.occupied,
        palette.focused,
        palette.urgent,
        palette.visible,
    ];
    for (bytes, color) in report[1..16].chunks_mut(3).zip(colors) {
        bytes.copy_from_slice(&color.0);
    }
    report[16] = (palette.dim.clamp(0.0, 1.0) * 255.0).round() as u8;
    report
}

/// Brightness of the 1-6 keys, shown in the keymap's width colour; the keymap
/// fades them out when reports stop coming.
pub fn width_report(levels: &[u8; WIDTH_KEY_COUNT]) -> Report {
    let mut report = [0; REPORT_LEN];
    report[0] = EVENT_WIDTH;
    report[1..=WIDTH_KEY_COUNT].copy_from_slice(levels);
    report
}

/// Whether the screen is being shared; the keymap strobes a key while it is.
pub fn cast_report(casting: bool) -> Report {
    let mut report = [0; REPORT_LEN];
    report[0] = EVENT_CAST;
    report[1] = casting.into();
    report
}

/// Whether the desktop is locked; the keymap breathes instead of its lighting.
pub fn lock_report(locked: bool) -> Report {
    let mut report = [0; REPORT_LEN];
    report[0] = EVENT_LOCK;
    report[1] = locked.into();
    report
}

/// The keys of the game/chat balance strip: the right half's top row, from the
/// split outwards. The first is all game, the middle the centre, the last all chat.
pub const BALANCE_KEY_COUNT: u8 = 7;

/// ChatMix balance as novamix counts it: -100 all game, 0 even, 100 all chat.
pub fn balance_report(balance: i32) -> Report {
    let mut report = [0; REPORT_LEN];
    report[0] = EVENT_BALANCE;
    report[1] = ((balance.clamp(-100, 100) + 100) * 255 / 200) as u8;
    report
}

/// The balance a keyboard `event_BALANCE_SET` packet asks for, if it is one.
pub fn balance_set(packet: &Report) -> Option<i32> {
    let key = i32::from(packet[1]);
    (packet[0] == EVENT_BALANCE_SET && key < i32::from(BALANCE_KEY_COUNT)).then(|| {
        (key * 2 - i32::from(BALANCE_KEY_COUNT) + 1) * 100 / (i32::from(BALANCE_KEY_COUNT) - 1)
    })
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum Playback {
    #[default]
    NoPlayer,
    Paused,
    Playing,
}

/// The desktop's media player, as the media keys show it.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct Media {
    pub playback: Playback,
    pub can_previous: bool,
    pub can_next: bool,
}

/// Colours of the previous, play/pause and next keys.
pub fn media_keys_report(
    media: &Media,
    palette: &Palette,
) -> Report {
    let skip = |can| {
        if can {
            palette.media_skip
        } else {
            palette.media_noskip
        }
    };
    let play = match media.playback {
        Playback::NoPlayer => palette.media_none,
        Playback::Paused => palette.media_paused,
        Playback::Playing => palette.media_playing,
    };
    let mut report = [0; REPORT_LEN];
    report[0] = EVENT_MEDIA_KEYS;
    let colors = [skip(media.can_previous), play, skip(media.can_next)];
    for (bytes, color) in report[1..10].chunks_mut(3).zip(colors) {
        bytes.copy_from_slice(&color.0);
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    // Golden vector, shared with tests/protocol_test.c in the keymap.
    #[test]
    fn test_workspaces_report() {
        use WorkspaceState::*;
        let slot = |state, dim| Slot { state, dim };
        let report = workspaces_report(&[
            slot(Empty, false),
            slot(Occupied, false),
            slot(Focused, false),
            slot(Urgent, false),
            slot(Visible, true),
            slot(Empty, true),
            slot(Occupied, true),
            slot(Urgent, true),
        ]);
        let mut want = [0; REPORT_LEN];
        want[..9].copy_from_slice(&[4, 0, 1, 2, 3, 0x84, 0x80, 0x81, 0x83]);
        assert_eq!(report, want);
    }

    // Golden vector, shared with tests/protocol_test.c in the keymap.
    #[test]
    fn test_palette_report() {
        let palette = Palette {
            empty: Rgb([1, 2, 3]),
            occupied: Rgb([4, 5, 6]),
            focused: Rgb([7, 8, 9]),
            urgent: Rgb([10, 11, 12]),
            visible: Rgb([13, 14, 15]),
            dim: 0.5,
            media_playing: Rgb([0, 0xff, 0]),
            media_paused: Rgb([0xff, 0, 0]),
            media_none: Rgb([0x30, 0x30, 0x30]),
            media_skip: Rgb([0, 0xff, 0x7f]),
            media_noskip: Rgb([0, 0, 0]),
        };
        let mut want = [0; REPORT_LEN];
        want[..17].copy_from_slice(&[5, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 128]);
        assert_eq!(palette_report(&palette), want);
    }

    // Golden vector, shared with tests/protocol_test.c in the keymap.
    #[test]
    fn test_media_keys_report() {
        let palette = Palette {
            media_playing: Rgb([4, 5, 6]),
            media_skip: Rgb([1, 2, 3]),
            media_noskip: Rgb([7, 8, 9]),
            ..Palette::default()
        };
        let media = Media {
            playback: Playback::Playing,
            can_previous: true,
            can_next: false,
        };
        let mut want = [0; REPORT_LEN];
        want[..10].copy_from_slice(&[6, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        assert_eq!(media_keys_report(&media, &palette), want);
        let idle = media_keys_report(&Media::default(), &palette);
        assert_eq!(idle[4..7], palette.media_none.0);
    }

    // Golden vector, shared with tests/protocol_test.c in the keymap.
    #[test]
    fn test_width_report() {
        let mut want = [0; REPORT_LEN];
        want[..7].copy_from_slice(&[7, 255, 102, 0, 7, 9, 200]);
        assert_eq!(width_report(&[255, 102, 0, 7, 9, 200]), want);
    }

    // Golden vectors, shared with tests/protocol_test.c in the keymap.
    #[test]
    fn test_balance_report() {
        for (balance, byte) in [(-100, 0), (0, 127), (100, 255), (-500, 0)] {
            let mut want = [0; REPORT_LEN];
            want[..2].copy_from_slice(&[8, byte]);
            assert_eq!(balance_report(balance), want);
        }
    }

    // Golden vectors, shared with tests/protocol_test.c in the keymap.
    #[test]
    fn test_cast_report() {
        assert_eq!(cast_report(true)[..3], [10, 1, 0]);
        assert_eq!(cast_report(false)[..3], [10, 0, 0]);
    }

    #[test]
    fn test_lock_report() {
        assert_eq!(lock_report(true)[..3], [11, 1, 0]);
        assert_eq!(lock_report(false)[..3], [11, 0, 0]);
    }

    #[test]
    fn test_balance_set() {
        let packet = |key| {
            let mut packet = [0; REPORT_LEN];
            packet[..3].copy_from_slice(&[9, key, 7]);
            packet
        };
        let steps: Vec<_> = (0..7).map(|key| balance_set(&packet(key))).collect();
        let want = [-100, -66, -33, 0, 33, 66, 100].map(Some);
        assert_eq!(steps, want);
        assert_eq!(balance_set(&packet(7)), None);
        assert_eq!(balance_set(&[3; REPORT_LEN]), None);
    }

    #[test]
    fn test_rgb_parse() {
        assert_eq!("#30ff0a".parse(), Ok(Rgb([0x30, 0xff, 0x0a])));
        assert_eq!("000001".parse(), Ok(Rgb([0, 0, 1])));
        assert!("#12345".parse::<Rgb>().is_err());
        assert!("#12345g".parse::<Rgb>().is_err());
        assert_eq!(Rgb([0x30, 0xff, 0x0a]).to_string(), "#30ff0a");
    }
}
