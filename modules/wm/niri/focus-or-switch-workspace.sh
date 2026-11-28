#!/usr/bin/env sh
# Focus the monitor owning workspace $1 if it isn't focused yet; only
# switch to the workspace itself once its monitor is already focused.
set -eu

ws="$1"

target_output=$(niri msg --json workspaces | jq -r --arg ws "$ws" '.[] | select(.name == $ws) | .output // empty')
focused_output=$(niri msg --json focused-output | jq -r '.name // empty')

if [ -z "$target_output" ] || [ "$target_output" = "$focused_output" ]; then
  niri msg action focus-workspace "$ws"
else
  niri msg action focus-monitor "$target_output"
fi
