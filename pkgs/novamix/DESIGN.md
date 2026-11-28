# novamix

ChatMix for the SteelSeries Arctis Nova Pro Wireless on PipeWire: the base station's dial
balances two sinks, GAME and CHAT; a third, GLOBAL, is never on the dial. A daemon first; a CLI, TUI and GUI talk to it later
over its control socket. Protocol facts and their sources: PROTOCOL.md.

## Crates

- `novamix`: what clients share. `protocol` (decode/encode the base station's messages),
  `mix` (`Mix { game, chat }`, the gain curve), `ipc` (socket messages and framing).
- `novamix-daemon`: the daemon.
- `novamix-cli`: the `novamix` binary (clap only for `--socket` and the trailing words;
  `command.rs` parses the keywords, `show.rs` prints).

## Daemon

Threads, no async:

- **Main thread**: the PipeWire main loop. It owns the `Engine` and the sinks and handles every
  `Event` from a `pipewire::channel`.
- **Device thread** (`hid`): opens interface 4 of `1038:12e0` with hidapi (hidraw), writes the
  Sonar icon and ChatMix enable, reads with a 500 ms timeout and forwards decoded reports. A read
  error means unplugged: it sends `Disconnected` and looks again every 2 s. On stop it writes
  both options off.
- **Socket thread**: one request per connection, answered by the main loop through a reply
  channel inside the event.

`Engine<A: Audio>` holds the state (connected, sinks ready, mix) and is the part under test,
with a fake `Audio` that records mixes.

## Sinks: why `pw-loopback` and native volumes

- Loading `libpipewire-module-loopback` into the daemon's own context would be the cleanest
  (the sinks would die with the process), but pipewire-rs 0.10 has no `load_module`, and the
  raw FFI is not worth it yet. `pw-loopback` is that same module in a small PipeWire tool,
  shipped with PipeWire, and it is what the Python app runs, so sink names and behaviour match.
- Volumes are set natively: the registry finds `input.GAME`/`input.CHAT`/`input.GLOBAL`, the
  daemon binds them and sets `channelVolumes` on their `Props`. GLOBAL is set to full volume
  when it appears and the dial never writes to it, so it follows only what is downstream: the
  headset's own master volume. No `pactl`, so no dependency on
  pipewire-pulse, and no process spawned per dial step.
- The sinks stay up while the base station is unplugged, so apps keep their routes; the volumes
  stay where the dial left them.

## Upstream: the sinks play to the default sink at start

Not tied to the Arctis, so any headphones work. With no `--output`, the three loopbacks play to
the default sink as it is when the daemon starts: the loopbacks do not exist yet, so even if the
user chose GLOBAL as default, WirePlumber is on its fallback, the highest-priority real sink
(the dotfiles give the headset `priority.session` 2000).

- If the default is one of our own sinks (leftover loopbacks after a SIGKILL, a race), the daemon
  does not loop into it: it waits for the registry's start-up listing (a core sync) and picks the
  real `Audio/Sink` with the highest `priority.session`, then `priority.driver`, and logs which
  and why. With no real sink yet, the first one to appear is taken.
- The upstream is remembered: `$XDG_STATE_HOME/novamix/upstream` holds one `node.name`, written
  atomically whenever the daemon settles on the user's sink (the default at start, or `output`;
  the highest-priority pick too when nothing was remembered). Order at start: `--output` > the
  default sink if it is a real one > the remembered sink if present > the highest-priority real
  sink. The log and `novamix output` say which rule picked it. This is for `take_default`:
  WirePlumber remembers GLOBAL as the default, so the next start's default is ours.
- `novamix output [SINK]` shows the upstream and how it was chosen, or retargets the three
  loopbacks live to a present real sink (a node.name or id; our own sinks are refused) and
  remembers it. Retargeting sets `target.object` on the default metadata for each loopback's
  playback stream, as `route` does for streams: the sinks never go down, so sound continues.
- The upstream is also the explicit target of the loopbacks' playback streams: once the three
  `output.*` nodes exist the daemon writes `target.node` (node id) and `target.object` (serial)
  for each on the default metadata, the pair pipewire-pulse writes for a move, so a mixer shows
  the real sink instead of "Default (GLOBAL)". The daemon binds the default metadata and watches
  `target.object` on those three nodes: a value it did not write is a user move in a mixer. A
  real sink is `novamix output` (all three retargeted, remembered, logged `upstream set from the
  mixer: <sink>`); one of our own sinks is put back, and so is the default sink when it is one
  of ours (pipewire-pulse writes -1 for a move to the default; a real default counts as chosen).
- Unplug re-picks as below but does not touch the remembered sink; the stand-in is marked, and
  when the remembered sink reappears while the stand-in plays, the loopbacks move back to it.
- The control socket's request frames start with a version byte (`ipc::VERSION`); a daemon that
  sees another one answers `client and daemon versions differ`. Response index 3 stays `Error`,
  so even a CLI from before the byte shows that message; a new CLI that is hung up on by an old
  daemon says the same.
- `--output <node.name>` pins the upstream as before (`default` means the same as no option).
- When a followed upstream disappears (unplug), the loopbacks are ended and started again into
  the default sink if it is a real one, else the best by the same rule; the running streams are
  swept again. Streams on our sinks meanwhile are moved by WirePlumber. A pinned upstream is not
  followed: its loopbacks stay and WirePlumber relinks them (untested).
- The dial and the HID part stay tied to the Arctis base station; without one the sinks still
  work, mixed from the CLI.
- The default sink is never changed, except by `take_default = true` (top level of config.toml,
  off by default): once the three sinks are bound, and the upstream was chosen, the daemon runs
  `wpctl set-default` on GLOBAL (whatever it is named). It is not undone on stop: WirePlumber
  falls back by priority.

## Routing: a WirePlumber script the daemon writes

New playback streams go to CHAT or GAME by rules on their PipeWire properties
(`application.name`, `application.process.binary`, `media.role`, any other). Unmatched streams go
to the default sink, which the daemon never changes (unless `take_default` asks): not at start, not at stop, no priority
tricks. The user picks it the normal way, and WirePlumber remembers it. To have every
unmatched stream at a fixed volume, make GLOBAL the default once with
`wpctl set-default <node id of input.GLOBAL>` (`novamix status` shows the id; pavucontrol works
too); making GAME or CHAT the default puts the rest on the dial. If the default is one of the
three sinks at start, the daemon logs it and changes nothing.

- Why not the daemon moving streams over its own connection: it sees a new stream at the same
  time as WirePlumber, which may link it to the default sink first, and the move is then a
  short blip on the wrong sink.
- Why not `stream.rules` alone: WirePlumber 0.5 reads them only for its restore options
  (`state.restore-*`); a `target.object` set there never reaches linking (read in
  `node/state-stream.lua` and `linking/find-defined-target.lua`, 0.5.19).
- So the daemon writes `$XDG_DATA_HOME/wireplumber/scripts/novamix.lua` (fixed, from the crate)
  and `$XDG_CONFIG_HOME/wireplumber/wireplumber.conf.d/90-novamix.conf` (the rules as
  `novamix.rules`, the component, `custom.novamix` in the `main` profile). The script hooks
  `node-added` after `node/restore-stream`, as restore-stream itself does for saved targets: it
  matches with WirePlumber's own matcher and puts `target.object` on the default metadata
  before the stream is first linked. Streams that name a target themselves are left alone.
- Rules match playback streams only (`media.class = Stream/Output/Audio` is added to each), so
  a microphone with `media.role = Communication` never records CHAT.
- Streams already playing are routed by the daemon, in Rust (`route_running` in `pw.rs`), not by
  the script: the script only sees `node-added` events, so a stream that started before the sinks
  existed (or before the daemon, with WirePlumber not restarted) would stay put, and the daemon
  already has the rules, the stream list and the move code of `route <stream> <sink>`. When the
  third sink is bound it matches each playback stream with `route::matching`, skips streams that
  name a target themselves, and moves the rest through `target.object` on the default metadata.
  It runs once per daemon start (a flag, never reset, so a rebound sink does not repeat it and
  never undoes a move made by hand). It logs one line per moved stream and `routed N running
  streams`.
- When either file changes the daemon runs `systemctl --user try-restart wireplumber`; WirePlumber
  then links the running streams again by the rules. With the daemon stopped the sinks are gone,
  the script finds no target and streams go where they would anyway.

Defaults: voice chat apps go to CHAT, music players (`media.role = Music`, mpd, Spotify, Feishin,
Supersonic) to GLOBAL so the dial leaves them alone, games to GAME. A `[[route]]` table of your
own replaces ALL the defaults, not just one: copy the defaults you still want from `DEFAULTS`
in `route.rs`. (mpv is not listed: it only matches when it sets `media.role = Music`.)

Config, `$XDG_CONFIG_HOME/novamix/config.toml` (or `--config`); its `[[route]]` tables replace the
defaults (`DEFAULTS` in `route.rs`). Within one `match` entry all properties must match, any
entry is enough; a value starting with `~` is a POSIX extended regular expression:

```toml
[[route]]
sink = "chat"
match = [{ "media.role" = "Communication" }, { "application.name" = "Discord" }]

[[route]]
sink = "game"
match = [{ "application.process.binary" = "~\\.exe$" }]
```

Sink names: an optional `[sinks]` table renames them (`game`, `chat`, `global`, each optional);
a config without it keeps `GAME`, `CHAT` and `GLOBAL`. The name drives the node name (`input.<name>`, what `pw-loopback -n` makes) and the
description shown in pavucontrol; the keywords `game` and `chat` in `[[route]]` and the CLI do not
change. Names are one word, different from each other. `sink = "global"` is a valid rule
target too (a rule can pull a stream back onto GLOBAL from a default that is on the dial).

```toml
[sinks]
game = "Games"
chat = "Voice"
global = "Everything"
```

Defaults: CHAT for `media.role = Communication` and Discord (and Canary/PTB), Vesktop,
WebCord, Mumble, TeamSpeak 3 and Yak VC by binary or app name; GAME for `media.role = Game`
(SDL), any `.exe` binary or app name (Wine, Proton), and the `wine*` and `gamescope` binaries.

## Control socket

`$XDG_RUNTIME_DIR/novamix.sock`, one request per connection, each message a little-endian `u32`
length then bincode 2 (standard config).

- `Request::Status` -> `Response::Status(Status { connected, sinks: [(Sink, node id, node.name)], mix })`
- `Request::Set(Mix)` -> `Response::Ok`: sets GAME and CHAT (never GLOBAL) until the dial moves again, and with
  `write_dial = true` (the default; top level of config.toml) writes it to the base station.
- `Request::Routes` -> `Response::Routes { rules, streams }`: the rules from the config and the
  playback streams alive now, each with the `node.name` it is linked to (from the registry's
  Link objects).
- `Request::Move(node id, Sink)` -> `Response::Ok`: runs `pw-metadata <id> target.object
  <sink's object.serial> Spa:Id`, the same key the WirePlumber script sets, so WirePlumber links
  the stream again. A manual move lasts until the stream is routed again (a restart of the
  app or of WirePlumber). Errors come back as `Response::Error`.

Later a `Subscribe` request that streams `Status` on every change for the TUI/GUI.

## CLI

iproute2 style: `novamix OBJECT [ARGS]`, keywords and no dashed flags except the global
`--socket`/`NOVAMIX_SOCKET`; `help` works as `help route` and `route help`.

- `status`: base station connected or not, the mix, each sink's node id.
- `mix`, `mix 70` (game 70%, chat 100%), `mix 70 40`, `mix balance -100..100` (the dial: positive
  is towards chat and lowers game), `mix reset`.
- `route [list]`: rules, and each live stream with its sink; `(no rule)` marks streams no rule
  matched, which play wherever the default sink sends them. The daemon decides this with the
  `regex` crate on the stream's properties (`~` values are regexes, close to but not exactly
  WirePlumber's POSIX ERE).
- `route <stream> chat|game|global`: the stream is a node id or a unique, case-insensitive part of its
  application name or binary; the CLI asks `Routes`, resolves it, sends `Move`.
