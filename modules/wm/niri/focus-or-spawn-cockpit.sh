#!/usr/bin/env sh
# Focus the Homun Emacs cockpit frame (title "Homun cockpit"), or with `here` bring it to
# the focused workspace; with none, open one from the container's Emacs daemon, laid out
# by claude-cockpit (transcript on top, compose box and board below, todos right).
set -eu

id=$(niri msg --json windows | jq -r '[.[] | select(.title == "Homun cockpit")][0].id // empty')

if [ -z "$id" ]; then
  exec "$HOME/.config/niri/bin/emacsclient" -s "$HOME/claude/.services/emacs/socket" -n -c \
    -F '((name . "Homun cockpit") (title . "Homun cockpit"))' --eval '(claude-cockpit)'
fi

if [ "${1:-}" = here ]; then
  ws=$(niri msg --json workspaces | jq -r '.[] | select(.is_focused) | .idx')
  niri msg action move-window-to-workspace --window-id "$id" --focus false "$ws"
fi
niri msg action focus-window --id "$id"
