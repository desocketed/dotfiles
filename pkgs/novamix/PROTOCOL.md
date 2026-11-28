# Arctis Nova Pro Wireless base station protocol

Every fact here comes from one source unless noted:

- **[D]** Dymstro, nova-chatmix-linux, 0BSD: README "Protocol description" and `nova-chatmix.py`,
  https://github.com/Dymstro/nova-chatmix-linux (read 2026-11-16; tested by its author on MCU
  firmware 01.29.27, DSP 00.03.82). Reverse engineered from Sonar's USB traffic.

No GPL project's code was read for this; HeadsetControl, SteelVoiceMix and Arctis Sound Manager
are not sources here.

| Fact | Value | Source |
| --- | --- | --- |
| USB vendor id | `0x1038` (SteelSeries) | [D] `NovaProWireless.VID` |
| Product id, base station | `0x12e0` | [D] `NovaProWireless.PID` |
| HID interface | `bInterfaceNumber` 4, OUT endpoint `0x04`, IN `0x84` | [D] README |
| Message length written | 63 bytes, zero padded (the README says 64; the working code writes 63) | [D] `MSGLEN` |
| Byte 0 | `0x06` host to base station, `0x07` base station to host | [D] README, `TX`/`RX` |
| Byte 1 | option, bytes 2.. its value | [D] README |
| `0x49` (73) | ChatMix enable, 1 byte 0/1: lets a press on the dial switch it between volume and ChatMix | [D] README, `OPT_CHATMIX_ENABLE` |
| `0x8d` (141) | Sonar icon on the base station's screen, 0/1 (the author thinks it is only the icon) | [D] README, `OPT_SONAR_ICON` |
| `0x45` (69) | ChatMix dial: byte 2 game, byte 3 chat, each 0-100; sent on every turn with both values | [D] README, `chatmix_volume_control` |
| `0x25` (37) | Volume attenuation, 0 loudest to 56 mute | [D] README |
| `0x2e` (46) | EQ preset 0-18 (4 = custom) | [D] README (not used yet) |
| `0x31` (49) | Custom EQ: band, value 0-40 (-20..20 dB on the station) | [D] README (not used yet) |
| On `Set` | write `0x06 0x45 game chat`, same layout as the report, to move the base's dial; implemented, awaiting a hardware check (UNCERTAIN.md) | [D] README hint |
| On exit | write `0x49 0` and `0x8d 0` to hand the dial back to plain volume | [D] `close()` |

Not from [D]:

- The PipeWire side mirrors [D]'s `ChatMix` class: one `pw-loopback -n GAME|CHAT
  --capture-props=media.class=Audio/Sink -P <headset sink>` each, headset sink found by
  `SteelSeries_Arctis_Nova_Pro_Wireless` in its name. The loopback's sink node is `input.<name>`
  ([D] sets volumes on `input.GAME`).
- Volume levels map to linear gain as `(level/100)^3`, the cubic mapping PulseAudio (and so
  pipewire-pulse behind `pactl set-sink-volume N%`, which [D] calls) uses:
  https://www.freedesktop.org/wiki/Software/PulseAudio/Documentation/Developer/Clients/WritingVolumeControlUIs/
- The gain is set as `SPA_PROP_channelVolumes` in a `Props` param on the node, PipeWire's own
  volume property: https://docs.pipewire.org/group__spa__param.html (spa/param/props.h).
