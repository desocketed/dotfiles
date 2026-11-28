# Record a region of the screen. The first run asks for the region and starts
# recording; the next run stops it and puts the file on the clipboard, ready to
# paste into a chat.
state="${XDG_RUNTIME_DIR:-/tmp}/niri-screenrecord"

if [ -f "$state" ]; then
  read -r pid file < "$state"
  rm -f "$state"
  kill -INT "$pid" 2>/dev/null || true
  # wf-recorder finishes the file after the signal
  while kill -0 "$pid" 2>/dev/null; do sleep 0.1; done
  printf 'file://%s\n' "$file" | wl-copy --type text/uri-list
  notify-send --expire-time 3000 "Recording saved" "$file (on the clipboard)" || true
  exit 0
fi

region=$(slurp) || exit 0
dir="${XDG_VIDEOS_DIR:-$HOME/Videos}/Recordings"
mkdir -p "$dir"
file="$dir/$(date '+%Y-%m-%d %H-%M-%S').mp4"
# yuv420p so browsers and chat apps play it inline
wf-recorder --geometry "$region" --pixel-format yuv420p --file "$file" &
echo "$! $file" > "$state"
notify-send --expire-time 2000 "Recording" "Press the key again to stop" || true
wait
