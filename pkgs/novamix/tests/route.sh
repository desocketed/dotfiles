#!/usr/bin/env bash
# Routing against a private PipeWire and WirePlumber with null sinks: no sound card, no
# desktop session touched. Run: nix shell nixpkgs#pipewire nixpkgs#wireplumber nixpkgs#jq nixpkgs#pulseaudio -c tests/route.sh
# The daemon runs for real: its three pw-loopbacks, GLOBAL as default sink, the CLI.
set -eu
daemon=${1:-target/debug/novamix-daemon}
dir=$(mktemp -d)
trap 'kill $(jobs -p) 2>/dev/null; [ -n "${KEEP:-}" ] && echo "$dir" || rm -rf "$dir"' EXIT
export XDG_CONFIG_HOME=$dir/config XDG_DATA_HOME=$dir/data XDG_STATE_HOME=$dir/state
export XDG_RUNTIME_DIR=$dir/run DBUS_SESSION_BUS_ADDRESS=disabled:
mkdir -m 700 "$XDG_RUNTIME_DIR"
mkdir -p "$XDG_CONFIG_HOME/pipewire/pipewire.conf.d"

# Renamed sinks: GAME stays, CHAT is Voice, GLOBAL is Main
mkdir -p "$XDG_CONFIG_HOME/novamix"
printf '[sinks]\nchat = "Voice"\nglobal = "Main"\n' > "$XDG_CONFIG_HOME/novamix/config.toml"

sink() { echo "{ factory = adapter args = { factory.name = support.null-audio-sink node.name = \"$1\" media.class = Audio/Sink audio.position = [ FL FR ] priority.session = $2 } }"; }
echo "context.objects = [ $(sink headset 2000) $(sink speakers 1000) ]" \
	> "$XDG_CONFIG_HOME/pipewire/pipewire.conf.d/sinks.conf"

# Writes the WirePlumber files, then fails to reach a PipeWire that is not running yet
"$daemon" > "$dir/daemon.log" 2>&1 || true
pipewire > "$dir/pw.log" 2>&1 &
sleep 1
wireplumber > "$dir/wp.log" 2>&1 &
sleep 2
pipewire-pulse > "$dir/pulse.log" 2>&1 &
sleep 1

# Each named after its app, so the links below show who went where
play() { pw-cat -p --raw -P "{ node.name = \"$1\" application.name = \"$1\" media.role = Test $2 }" /dev/zero & }

# Playing before the daemon and its sinks exist: only the daemon's start-up sweep can route it
play early.exe ''
play quiet ''
pw-cat -p --raw -P '{ node.name = "mpd" application.name = "Music Player Daemon" media.role = Test }' /dev/zero &
sleep 1

# A leftover from a daemon that was killed: a sink of ours, the highest priority, the default.
# The daemon must not loop into it but pick the highest-priority real sink, headset (2000,
# not speakers, 1000), and say so
nodeid() { pw-dump | jq -r ".[] | select(.info.props.\"node.name\" == \"$1\") | .id"; }
pw-loopback -n GAME --capture-props='media.class=Audio/Sink node.description=leftover priority.session=3000' -P speakers > "$dir/leftover.log" 2>&1 &
leftover=$!
sleep 2
wpctl set-default "$(nodeid input.GAME)"
"$daemon" > "$dir/daemon.log" 2>&1 &
daemon_pid=$!
sleep 3
kill $leftover
sleep 1

# The default sink is the user's choice, as with pavucontrol; the daemon never sets it
wpctl set-default "$(nodeid headset)"

play Discord ''
play firefox ''
play game.exe ''
play voip 'media.role = "Communication"'
sleep 2

fail=0
expect() {
	if pw-link -l | grep -A8 "^$2:playback_FL" | grep -q "<- $1:output_FL"; then
		echo "ok   $1 -> $2"
	else
		echo "FAIL $1 -> $2"
		fail=1
	fi
}
grep -q "upstream is headset (highest priority real sink)" "$dir/daemon.log" && echo "ok   fallback picked headset" || { echo "FAIL fallback did not pick headset"; fail=1; }
if pw-dump | jq -e '[.[] | select(.type == "PipeWire:Interface:Link")] | length > 0' >/dev/null; then
	if pw-link -l | grep -B1 -A3 "^headset:playback_FL" | grep -q "output.GAME:output_FL"; then echo "ok   output.GAME -> headset"; else echo "FAIL output.GAME is not linked to headset"; fail=1; fi
fi
expect early.exe input.GAME
expect mpd input.Main
grep -q "routed 2 running streams" "$dir/daemon.log" && echo "ok   sweep logged" || { echo "FAIL sweep not logged"; fail=1; }
expect Discord input.Voice
expect voip input.Voice
expect game.exe input.GAME
# Unmatched streams stay on the default sink, which the daemon leaves alone
if pw-metadata -n default 0 default.audio.sink | grep -q headset; then
	echo "ok   default sink is still headset"
else
	echo "FAIL the default sink changed"
	fail=1
fi
expect firefox headset
expect quiet headset
cli=$(dirname "$daemon")/novamix
list=$("$cli" route list)
echo "$list" | grep Discord | grep -q "no rule" && { echo "FAIL Discord marked no rule"; fail=1; } || echo "ok   Discord has a rule"
echo "$list" | grep firefox | grep -q "no rule" && echo "ok   firefox marked no rule" || { echo "FAIL firefox not marked"; fail=1; }
"$cli" status | grep -q "input.Main node" && echo "ok   status lists Main" || { echo "FAIL status lists Main"; fail=1; }
"$cli" route firefox global > /dev/null
sleep 1
expect firefox input.Main
"$cli" route firefox game > /dev/null
sleep 1
expect firefox input.GAME

# One way to a fixed-volume default: wpctl set-default once; new unmatched streams follow it
wpctl set-default "$("$cli" status | sed -n 's/.*input.Main node //p')"
play mpv ''
sleep 2
expect mpv input.Main

# The dial never touches Main: it stays at full volume while GAME and CHAT are set
"$cli" mix 30 20 > /dev/null
sleep 1
vol() { wpctl get-volume "$(nodeid "$1")" | cut -d' ' -f2; }
for name in input.Main:1.00 input.GAME:0.30 input.Voice:0.20; do
	if [ "$(vol "${name%:*}")" = "${name#*:}" ]; then echo "ok   ${name%:*} volume ${name#*:}"; else echo "FAIL ${name%:*} volume is $(vol "${name%:*}")"; fail=1; fi
done

# take_default = true: after the sinks are bound GLOBAL (Main) becomes the default; the upstream is
# still the real sink that was the default at start
kill $daemon_pid
wait $daemon_pid 2>/dev/null || true
sleep 1
wpctl set-default "$(nodeid headset)"
printf 'take_default = true\n[sinks]\nchat = "Voice"\nglobal = "Main"\n' > "$XDG_CONFIG_HOME/novamix/config.toml"
"$daemon" > "$dir/daemon2.log" 2>&1 &
daemon_pid=$!
sleep 4
grep -q "upstream is headset (the default sink at start)" "$dir/daemon2.log" && echo "ok   upstream is headset (take_default)" || { echo "FAIL upstream not headset"; fail=1; }
if pw-metadata -n default 0 default.audio.sink | grep -q '"name":"input.Main"'; then echo "ok   default sink is GLOBAL (take_default)"; else echo "FAIL default is not GLOBAL"; fail=1; fi

# A remembered sink: default is one of ours at start (as after take_default), the state file
# names speakers; speakers wins over the higher-priority headset
stop_daemon() { kill $daemon_pid; wait $daemon_pid 2>/dev/null || true; sleep 1; }
state=$XDG_STATE_HOME/novamix/upstream
start_with_ours_as_default() {
	pw-loopback -n GAME --capture-props='media.class=Audio/Sink node.description=leftover priority.session=3000' -P speakers > "$dir/leftover.log" 2>&1 &
	leftover=$!
	sleep 2
	wpctl set-default "$(nodeid input.GAME)"
	"$daemon" > "$1" 2>&1 &
	daemon_pid=$!
	sleep 3
	kill $leftover
	sleep 1
}
linked() { pw-link -l | grep -A3 "^$1:playback_FL" | grep -q "output.GAME:output_FL"; }
stop_daemon
printf 'speakers\n' > "$state"
start_with_ours_as_default "$dir/daemon3.log"
grep -q "upstream is speakers (the remembered sink)" "$dir/daemon3.log" && echo "ok   remembered sink picked over the headset" || { echo "FAIL remembered sink not picked"; fail=1; }
linked speakers && echo "ok   output.GAME -> speakers" || { echo "FAIL output.GAME is not on speakers"; fail=1; }
[ "$(cat "$state")" = speakers ] && echo "ok   state is speakers" || { echo "FAIL state is $(cat "$state")"; fail=1; }

# output <sink>: live, remembered, our own sinks refused
"$cli" output | grep -q "^speakers (the remembered sink)" && echo "ok   output shows speakers" || { echo "FAIL output does not show speakers"; fail=1; }
"$cli" output headset > /dev/null
sleep 2
linked headset && echo "ok   output headset moved the loopbacks" || { echo "FAIL output.GAME is not on headset"; fail=1; }
[ "$(cat "$state")" = headset ] && echo "ok   state is headset" || { echo "FAIL state is $(cat "$state")"; fail=1; }
"$cli" output | grep -q "^headset (output command)" && echo "ok   output shows how" || { echo "FAIL output after set"; fail=1; }
"$cli" output input.Voice 2>&1 | grep -q "novamix's own" && echo "ok   own sink refused" || { echo "FAIL own sink not refused"; fail=1; }
"$cli" output nosuchsink 2>&1 | grep -q "not a real sink" && echo "ok   unknown sink refused" || { echo "FAIL unknown sink not refused"; fail=1; }
"$cli" output "$(nodeid speakers)" > /dev/null
sleep 2
linked speakers && echo "ok   output by node id" || { echo "FAIL output by id"; fail=1; }

# What pavucontrol does: the loopbacks' streams carry the upstream as an explicit target
# (pipewire-pulse's own move writes target.node and target.object), and a move in the mixer is
# `novamix output`. The Playback tab's device is the stream's sink-input
stream_of() { pactl list sink-inputs | awk -v n="$1" '/^Sink Input/{id=substr($3, 2)} $0 ~ "node.name = \"" n "\""{print id}'; }
target() { pw-metadata -n default "$(nodeid "$1")" target.object | sed -n "s/.*key:'target.object' value:'\([0-9]*\)'.*/\1/p" | tail -1; }
serial() { pw-dump | jq -r ".[] | select(.info.props.\"node.name\" == \"$1\") | .info.props.\"object.serial\""; }
for out in output.GAME output.Main output.Voice; do
	[ "$(target $out)" = "$(serial speakers)" ] && echo "ok   $out targets speakers explicitly" || { echo "FAIL $out has no explicit target speakers: $(target $out)"; fail=1; }
done
[ "$(pactl list sink-inputs | grep -c 'Sink: '"$(serial speakers)")" -ge 3 ] && echo "ok   pactl shows the loopbacks on speakers" || { echo "FAIL pactl does not show them on speakers"; fail=1; }
pactl move-sink-input "$(stream_of output.Main)" headset
sleep 2
linked headset && echo "ok   a move of output.Main in the mixer moved all three" || { echo "FAIL the mixer move did not reach output.GAME"; fail=1; }
[ "$(target output.Voice)" = "$(serial headset)" ] && echo "ok   output.Voice targets headset" || { echo "FAIL output.Voice target $(target output.Voice)"; fail=1; }
[ "$(cat "$state")" = headset ] && echo "ok   the mixer's choice is remembered" || { echo "FAIL state is $(cat "$state")"; fail=1; }
grep -q "upstream set from the mixer: headset" "$dir/daemon3.log" && echo "ok   mixer move logged" || { echo "FAIL mixer move not logged"; fail=1; }
pactl move-sink-input "$(stream_of output.Voice)" speakers
sleep 2
linked speakers && [ "$(cat "$state")" = speakers ] && echo "ok   a move of output.Voice moved all three" || { echo "FAIL move of output.Voice"; fail=1; }
pactl move-sink-input "$(stream_of output.GAME)" input.GAME
sleep 2
linked speakers && [ "$(target output.GAME)" = "$(serial speakers)" ] && echo "ok   a move to our own sink was reverted" || { echo "FAIL move to input.GAME not reverted"; fail=1; }
grep -q "moved to a sink of novamix's own: moved back" "$dir/daemon3.log" && echo "ok   revert logged" || { echo "FAIL revert not logged"; fail=1; }

# A client of another version: a frame without the version byte (the first protocol)
reply=$(python3 -I - "$XDG_RUNTIME_DIR/novamix.sock" <<'PY'
import socket, sys
s = socket.socket(socket.AF_UNIX)
s.connect(sys.argv[1])
s.sendall(b"\x01\x00\x00\x00\x00")
print(s.recv(4096))
PY
)
echo "$reply" | grep -q "versions differ" && echo "ok   version mismatch said so" || { echo "FAIL version mismatch: $reply"; fail=1; }

# The remembered sink is not there at start: a stand-in plays, the state is kept, and the
# sink comes back live
stop_daemon
printf 'dyn\n' > "$state"
start_with_ours_as_default "$dir/daemon4.log"
grep -q "upstream is headset (highest priority real sink)" "$dir/daemon4.log" && echo "ok   stand-in is headset" || { echo "FAIL stand-in not headset"; fail=1; }
[ "$(cat "$state")" = dyn ] && echo "ok   stand-in did not overwrite the state" || { echo "FAIL state is $(cat "$state")"; fail=1; }
# A sink that appears later; it lives as long as this loopback does
pw-loopback -n dynloop --capture-props='media.class=Audio/Sink node.name=dyn priority.session=10' -P speakers > "$dir/dyn.log" 2>&1 &
sleep 3
linked dyn && echo "ok   loopbacks went back to dyn" || { echo "FAIL output.GAME is not on dyn"; fail=1; }
grep -q "the remembered sink dyn is back" "$dir/daemon4.log" && echo "ok   reattach logged" || { echo "FAIL reattach not logged"; fail=1; }
exit $fail
