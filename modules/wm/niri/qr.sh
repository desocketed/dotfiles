# Select a region, decode the QR code in it and put the text on the clipboard.
set -euo pipefail

region=$(slurp) || exit 0
if text=$(grim -g "$region" - | zbarimg --quiet --raw -); then
  printf '%s' "$text" | wl-copy
  notify-send --expire-time 3000 "QR code copied" "$text" || true
else
  notify-send --expire-time 3000 "No QR code found" || true
fi
