# Uncertain

- The upstream is the default sink at start, or the highest-priority real sink when the default is
  one of ours. If that is the wrong sink, pass `--output <node.name>`.
- When a followed upstream is unplugged the loopbacks are respawned into another sink (so the
  sinks vanish and return for a moment, and streams on them are moved by WirePlumber, then swept
  again by the rules). Untested on hardware, and in `route.sh`, which cannot unplug a null sink.
  A replug does not move the upstream back; restart the daemon. A pinned `--output` is not
  followed at all: whether WirePlumber relinks its loopbacks after a replug is untested.
- `take_default`: `wpctl set-default` runs once when the sinks are bound; untested on the desktop
  with the dotfiles' priorities. Not restored on stop.
- The start-up routing of running streams runs when the third sink is bound, with the streams
  the registry listed by then. Running streams have lower ids than the new sinks, so they are
  listed first; a stream that appears in between is routed by the script instead. It is checked
  in `tests/route.sh` on a private PipeWire, not on a desktop with many streams.
- A daemon killed with SIGKILL leaves its three `pw-loopback`s (and so the sinks) behind.
- Writing `0x45` back to the base station on a `Set` from the socket (`write_dial`, default on) is
  implemented but untested on hardware: [D]'s README hints the base takes it; whether its display
  follows, and whether it echoes the write back (an equal report within 300 ms is ignored), awaits
  the user's check.
- Routing default rules are guesses not checked against real streams: Discord/Vesktop under
  Electron may report `application.process.binary = electron` (the name rule still catches
  `Discord`/`vesktop`); Yak VC's names (`yakvc`, `Yak VC`) are assumed; native Steam games
  without SDL have no shared property and stay on the default sink. Check with `pw-dump` or
  `wpctl inspect <id>` and add rules.
- Browsers in a call are not detected: Firefox and Chromium give WebRTC playback no
  `media.role`, so calls in a browser go to the default sink.
- A matched app moved by hand (pavucontrol) is sent back by the rule the next time it starts.
- The script uses WirePlumber 0.5's Lua API; 0.4 ignores the `.conf` fragment.
- The daemon never sets the default sink unless `take_default` is on (the user's rule; WirePlumber remembers the user's choice).
  A default left on GAME, CHAT or GLOBAL by an earlier run or by hand stays; with the daemon
  stopped (the sinks gone) WirePlumber falls back by itself, untested on the desktop. At start
  the daemon only logs when the default is one of its sinks, and `--output default` refuses
  such a default. Apps that follow "System Default" while it is GAME play on the dial: that is
  the user's setting, `wpctl set-default` changes it.
- GLOBAL "follows only the master volume" because it is never written to and sits upstream of
  the headset's sink; whether the base station's volume knob changes that sink's volume (and not
  only the headset's own gain) is untested on hardware.
- `(no rule)` in `route list` is decided with Rust's `regex`, WirePlumber matches with POSIX ERE:
  exotic patterns (back-references, POSIX classes in odd forms) may differ. The default rules do not.
- Renamed sinks: names with characters PipeWire or WirePlumber treat specially (dots, quotes)
  are untested; the daemon only refuses empty names and spaces. Streams moved by hand to a sink
  stay there until the stream is routed again, as before.
- `output` and the reattach move the loopbacks with `target.object` on the default metadata
  (tested on a private PipeWire and WirePlumber 0.5 only); WirePlumber setting `target.object`
  on a stream whose first target is gone is untested on the desktop. A pinned (`--output`)
  upstream is neither remembered nor reattached, but `output` can still move it.
- The remembered sink is a `node.name`; a device whose name changes (another USB port) counts as a
  different sink.
- The mixer's device for the loopbacks' streams is checked with `pactl` and the default metadata
  on a private PipeWire, not in pavucontrol itself: which of `target.node`, `target.object` or
  the stream's own `target.object` property a given pavucontrol version reads for its "Default"
  entry is untested. A move to the default sink is read as a choice only when the default is a
  real sink; moves are followed one at a time (pw-metadata per write), so a mixer may flicker.
