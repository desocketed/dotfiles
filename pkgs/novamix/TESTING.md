# Testing on the desktop

1. Stop the Python app: it runs with `uv`, not as a systemd unit, so end that `uv run` process
   (C-c in its terminal, or `pkill -f nova-chatmix`). It detaches the kernel's HID driver from
   the base station's interface 4 and leaves it detached, so unplug and replug the base station
   after it stops. Check: `wpctl status` shows no GAME/CHAT/GLOBAL.
2. The daemon needs read/write on the base station's hidraw node. The Python app's udev rule
   (`50-nova-pro-wireless.rules`) already grants it; `udev/70-novamix.rules` is the same rule.
3. From the worktree: `nix develop -c cargo run -p novamix-daemon` (or `RUST_LOG=debug ...` to
   see every dial step). Expected log: `routing GAME, CHAT and GLOBAL to ...`, three
   `sink input... is node N` lines, `base station connected`. The Sonar icon shows on the
   base station.
4. Press the dial to switch it to ChatMix and turn it. In `pavucontrol` (Output Devices) or
   `watch wpctl status`, GAME drops as you turn towards chat and CHAT drops the other way;
   both are at 100% in the middle. GLOBAL stays at 100% whatever the dial does; its level moves
   only with the headset's own volume (the master).
5. Unplug and replug the base station: the log shows `disconnected`, then `connected`, and the
   dial works again.
6. Ctrl+C: GAME, CHAT and GLOBAL disappear and the dial is plain volume again.

## Stream routing

`tests/route.sh` checks the rules in the container on a private PipeWire with null sinks. On the
desktop, with WirePlumber 0.5 (`wireplumber --version`):

7. Start the daemon (`nu ~/claude/novamix/try.nu`). The first start logs `wrote
   ~/.local/share/wireplumber/scripts/novamix.lua`, `wrote
   ~/.config/wireplumber/wireplumber.conf.d/90-novamix.conf` and `routing rules changed,
   restarting WirePlumber`; a second start logs none of these.
8. Start Discord or Vesktop and play something in it (a call, or the voice settings' test).
   `wpctl status` lists it under Streams going to `input.CHAT` (pavucontrol, Playback tab:
   CHAT), and `journalctl --user -u wireplumber | grep novamix` shows `-> input.CHAT`.
   Turning the dial towards game makes it quieter.
9. Start a Proton game: it plays on GAME. Firefox, mpv and the rest stay on whatever your
   default sink is: the daemon never changes the default. If it is GAME or CHAT, the log says so
   at start.
10. Rules of your own: put `[[route]]` tables in `~/.config/novamix/config.toml` (DESIGN.md has
    the format) and restart the daemon; it restarts WirePlumber once, and the new rules hold for
    streams already playing too.
10a. Streams that are already playing: start Firefox playing something (it matches no rule by
    default, so use a game, Discord or a `media.role = Game` player such as `mpv
    --audio-client-name=x` with a rule of your own; or mpd/mpv under a `[[route]]` for its
    `application.name`) before the daemon, then restart it. The log says `routed running stream
    ... to ...` per stream and `routed N running streams`; `novamix route list` shows the
    stream on its rule's sink. Streams no rule matches stay where they were. Move one by hand
    (`novamix route <stream> game`) and restart nothing: it stays there.
11. To remove the routing: delete the two files from step 7 and
    `systemctl --user restart wireplumber`.

## CLI

`~/claude/.bin/novamix-update` installs `novamix` next to the daemon and restarts the service.
Then, in any terminal:

12. `novamix help`, `novamix route help` and `novamix help route` print help. With the daemon
    stopped, `novamix status` says it cannot reach the daemon.
13. `novamix status`: base station connected, the mix as the dial has it, and GAME, CHAT and GLOBAL with
    node ids that match `wpctl status`.
14. `novamix mix 50`: GAME drops to 50% in `wpctl status`/pavucontrol and chat stays at 100%.
    `novamix mix balance -60` lowers CHAT to 40%, `novamix mix reset` restores both. Turning
    the dial takes over again.
15. Play something in Firefox, then `novamix route list`: the rules, and Firefox's stream with
    its sink (`headset`'s node name). `novamix route firefox chat` moves it: the sound gets
    quieter if you turn the dial towards game, and `route list` shows `input.CHAT`;
    `novamix route firefox game` moves it back.
16. `novamix route firefox global` moves it to GLOBAL (`route list` shows `input.GLOBAL`); the
    dial no longer changes its volume. `novamix route firefox game` moves it back.
17. Fixed-volume default: `wpctl set-default N` with the node id of `input.GLOBAL` from
    `novamix status`. Start something new (mpv, a browser tab): it plays on GLOBAL and the dial
    leaves it alone, while the headset's volume knob still changes it. Restarting the daemon does
    not touch the default; to go back, `wpctl set-default` your headset or speakers.
18. `route list` marks streams that no rule matched (`no rule`), so you can see which ones follow
    the default sink.

## Any headphones

`tests/route.sh` checks the upstream pick and `take_default` on a private PipeWire. On the desktop:

18a. Make GLOBAL the default (`wpctl set-default <id of input.GLOBAL>`), stop the daemon and start
    it again. The log says `upstream is the default sink ...` (the headset, or whatever
    WirePlumber fell back to after the sinks went away) or, if the default was ours,
    `upstream is <headset> (highest priority real sink)`. `wpctl status` shows the three sinks
    and, in `pw-link -l`, `output.GAME` linked to your headphones. Sound still plays on them.
18b. Unplug the headset's USB (or switch the base station off): the log shows `upstream ... is
    gone; picking another` and the sinks come back on the next best sink (speakers). Plug it back:
    nothing moves until the next restart (the speakers stay the upstream), and a restart goes
    back to the headset.
18c. `take_default = true` at the top of `config.toml`, restart: the log says `default sink is now
    GLOBAL`, `wpctl status` marks GLOBAL with `*`, and the upstream is still the headset. Without
    the line the default is not touched. Stopping the daemon does not restore it.

18d. With `take_default = true` and GLOBAL the default, stop the daemon and start it again: the log
    says `upstream is <your headset> (the remembered sink)` and `cat
    ~/.local/state/novamix/upstream` shows it. `novamix output` shows it and how it was chosen.
    `novamix output <other sink's node.name>` (see `wpctl status`, or `wpctl inspect <id>`): sound
    moves to that sink without the sinks restarting (a brief gap at most), and a restart
    keeps it. `novamix output input.GAME` is refused. With an old CLI against the new daemon (or
    the reverse) the error is `client and daemon versions differ`.
18f. Open pavucontrol, Playback tab: the three `output.*` streams (named after the sinks, e.g.
    `output.GAME`) show your headset as their device, not "Default (...)". Choose another real
    sink on any one of them: all three follow, sound moves, and the log says `upstream set from
    the mixer: <sink>`; `novamix output` shows it and a restart keeps it. Choose one of
    GAME/CHAT/GLOBAL (or "Default" while the default is GLOBAL): the stream goes straight back
    and the log says why.
18e. Unplug the headset: the stand-in plays as in 18b, but the state file still names the headset.
    Plug it back: the log says `the remembered sink ... is back` and the sound moves back.

## Names

19. Put `[sinks]` with `game = "Games"`, `chat = "Voice"` and `global = "Everything"` in
    `~/.config/novamix/config.toml` and restart the daemon. pavucontrol and `wpctl status` show
    those names, `novamix status` shows `input.Games`, `input.Voice` and `input.Everything`, and
    the dial, the rules and `route ... chat|game|global` work as before. Remove the section to
    get GAME, CHAT and GLOBAL back; an old config without the section needs no change.
