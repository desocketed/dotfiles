#!/usr/bin/env sh
# Focus the Homun window (app id "homun"), or with `here` bring it to the focused workspace;
# with no Homun window, open one: WezTerm running `homun`.
set -eu

id=$(niri msg --json windows | jq -r '[.[] | select(.app_id == "homun")][0].id // empty')

if [ -z "$id" ]; then
  export PATH="$HOME/.config/niri/bin:$HOME/.nix-profile/bin:/nix/var/nix/profiles/default/bin:$HOME/.local/bin:$PATH"
  exec wezterm start --class homun -- homun
fi

if [ "${1:-}" = here ]; then
  ws=$(niri msg --json workspaces | jq -r '.[] | select(.is_focused) | .idx')
  niri msg action move-window-to-workspace --window-id "$id" --focus false "$ws"
fi
niri msg action focus-window --id "$id"
