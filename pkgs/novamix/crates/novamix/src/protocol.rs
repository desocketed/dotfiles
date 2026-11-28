//! The Arctis Nova Pro Wireless base station's HID messages (PROTOCOL.md has the sources).

use crate::mix::Mix;

/// SteelSeries.
pub const VID: u16 = 0x1038;

/// The Arctis Nova Pro Wireless base station.
pub const PID: u16 = 0x12e0;

/// The USB interface that carries settings and the dial; the others are audio and media keys.
pub const INTERFACE: i32 = 4;

/// Every message the host writes is this long, zero padded.
pub const MESSAGE_LEN: usize = 63;

/// First byte of a message the host sends.
const TX: u8 = 0x06;

/// First byte of a message the base station sends.
const RX: u8 = 0x07;

/// Lets the dial switch between volume and ChatMix (pressing it toggles).
const OPT_CHATMIX_ENABLE: u8 = 0x49;

/// Shows the Sonar icon on the base station's screen.
const OPT_SONAR_ICON: u8 = 0x8d;

/// The dial's volume attenuation, 0 (loudest) to 56 (mute).
const OPT_VOLUME: u8 = 0x25;

/// The ChatMix dial: game then chat volume, each 0-100.
const OPT_CHATMIX: u8 = 0x45;

/// What the base station reports.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Report {
    ChatMix(Mix),
    /// Attenuation, 0 (loudest) to 56 (mute).
    Volume(u8),
    /// Any option this crate does not read yet.
    Other(u8),
}

/// The report in one message read from the base station, `None` when it is not one.
pub fn decode(message: &[u8]) -> Option<Report> {
    let [RX, option, rest @ ..] = message else {
        return None;
    };
    match (*option, rest) {
        (OPT_CHATMIX, [game, chat, ..]) => Some(Report::ChatMix(Mix::new(*game, *chat))),
        (OPT_VOLUME, [attenuation, ..]) => Some(Report::Volume(*attenuation)),
        (OPT_CHATMIX, _) | (OPT_VOLUME, _) => None,
        (option, _) => Some(Report::Other(option)),
    }
}

/// The message that turns ChatMix on the dial on or off.
pub fn chatmix_enable(on: bool) -> [u8; MESSAGE_LEN] {
    encode(OPT_CHATMIX_ENABLE, on as u8)
}

/// The message that shows or hides the Sonar icon.
pub fn sonar_icon(on: bool) -> [u8; MESSAGE_LEN] {
    encode(OPT_SONAR_ICON, on as u8)
}

/// The message that moves the dial's ChatMix to `mix`, laid out as the base station reports it.
pub fn chatmix(mix: Mix) -> [u8; MESSAGE_LEN] {
    let mut message = encode(OPT_CHATMIX, mix.game);
    message[3] = mix.chat;
    message
}

fn encode(
    option: u8,
    value: u8,
) -> [u8; MESSAGE_LEN] {
    let mut message = [0; MESSAGE_LEN];
    message[0] = TX;
    message[1] = option;
    message[2] = value;
    message
}

#[cfg(test)]
mod test {
    use super::*;

    /// A ChatMix report as hidapi returns it: 63 bytes, game 100, chat 35.
    fn recorded_chatmix() -> Vec<u8> {
        let mut message = vec![0; MESSAGE_LEN];
        message[..4].copy_from_slice(&[0x07, 0x45, 100, 35]);
        message
    }

    #[test]
    fn test_decode_chatmix() {
        assert!(decode(&recorded_chatmix()) == Some(Report::ChatMix(Mix::new(100, 35))));
    }

    #[test]
    fn test_decode_volume() {
        assert!(decode(&[0x07, 0x25, 12, 0]) == Some(Report::Volume(12)));
    }

    #[test]
    fn test_decode_other_option() {
        assert!(decode(&[0x07, 0x2e, 4]) == Some(Report::Other(0x2e)));
    }

    #[test]
    fn test_decode_rejects_echo_and_short() {
        // Our own TX direction byte, a truncated ChatMix report and an empty read
        assert!(decode(&[0x06, 0x45, 100, 100]).is_none());
        assert!(decode(&[0x07, 0x45, 100]).is_none());
        assert!(decode(&[]).is_none());
    }

    #[test]
    fn test_decode_clamps_out_of_range() {
        assert!(decode(&[0x07, 0x45, 255, 101]) == Some(Report::ChatMix(Mix::new(100, 100))));
    }

    #[test]
    fn test_encode() {
        let message = chatmix_enable(true);
        assert!(message[..3] == [0x06, 0x49, 1]);
        assert!(message[3..].iter().all(|byte| *byte == 0));
        assert!(sonar_icon(false)[..3] == [0x06, 0x8d, 0]);
        let message = chatmix(Mix::new(40, 100));
        assert!(message[..4] == [0x06, 0x45, 40, 100]);
        assert!(message[4..].iter().all(|byte| *byte == 0));
    }
}
