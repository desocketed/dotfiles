{ config, pkgs, lib, ... }:
let
  cfg = config.modules.terminal;
in {
  options.modules.terminal.enable = lib.mkEnableOption "foot and wezterm terminals";

  config = lib.mkIf cfg.enable {
    programs.foot = {
      enable = true;
      settings = {
        main = {
          shell = "zsh";
          # Fallbacks in order: Noto cover the symbols JetBrains Mono lacks
          font = "JetBrainsMono:size=12, Noto Sans Math:size=12, Noto Sans Symbols:size=12, Noto Sans Symbols 2:size=12, Symbols Nerd Font:size=12";
        };
        colors = {
          foreground = "dcdccc";
          background = "111111";
        };
      };
    };

    xdg.enable = true;
    programs.wezterm = {
      enable = true;
      # OpenGL renderer: the host's drivers through nixGL
      package = config.lib.nixGL.wrap pkgs.wezterm;
      extraConfig = ''
        local config = wezterm.config_builder()

        config.default_prog = { "${config.home.homeDirectory}/.nix-profile/bin/nu" }
        config.font = wezterm.font_with_fallback({
          "JetBrains Mono",
          "Noto Sans Math",
          "Noto Sans Symbols",
          "Noto Sans Symbols 2",
          "Symbols Nerd Font",
        })
        config.font_size = 12

        -- niri is Wayland only; OpenGL goes through nixGL
        config.enable_wayland = true
        config.front_end = "OpenGL"

        config.colors = {
          foreground = "#dcdccc",
          background = "#111111",
          -- Explicit cursor colours so it stays visible under Zellij
          cursor_bg = "#dcdccc",
          cursor_fg = "#111111",
          cursor_border = "#dcdccc",
        }

        -- Chromeless: Zellij provides the tabs and panes
        config.window_decorations = "NONE"
        config.window_padding = { left = 0, right = 0, top = 0, bottom = 0 }
        config.window_close_confirmation = "NeverPrompt"
        config.audible_bell = "Disabled"

        config.enable_kitty_keyboard = true
        config.enable_kitty_graphics = true
        -- OSC 52 clipboard writes are on by default (no option to toggle)
        config.hyperlink_rules = wezterm.default_hyperlink_rules()

        config.keys = {
          { key = "Enter", mods = "ALT", action = wezterm.action.SpawnWindow },
          -- Stand-in for Alacritty's vi mode: vim-style copy mode
          { key = "Space", mods = "CTRL|SHIFT", action = wezterm.action.ActivateCopyMode },
        }

        -- Copy-mode indicator in the tab bar's right status. Not set_config_overrides: that
        -- reloads the config, which clears the key-table stack and drops copy mode mid-select
        config.hide_tab_bar_if_only_one_tab = false
        config.use_fancy_tab_bar = false
        config.tab_bar_at_bottom = true
        config.status_update_interval = 100
        -- No tabs or new-tab button: the bar is only a status line
        config.show_tabs_in_tab_bar = false
        config.show_new_tab_button_in_tab_bar = false
        wezterm.on("update-status", function(window, pane)
          -- Active pane's host and directory (from OSC 7; blank when the shell doesn't send it)
          local cwd, host = "", wezterm.hostname()
          local url = pane:get_current_working_dir()
          if url then
            cwd = (url.file_path or tostring(url)):gsub("^" .. wezterm.home_dir, "~")
            if url.host and url.host ~= "" then host = url.host end
          end
          -- Running command: the foreground process, or under Zellij its title for the focused pane
          local cmd = pane:get_foreground_process_name() or ""
          cmd = cmd:match("([^/]+)$") or cmd
          if cmd == "zellij" then cmd = pane:get_title() end
          -- Left, where the tabs were: copy mode while it is on, then the running command
          local left = {}
          if window:active_key_table() == "copy_mode" then
            left = {
              { Background = { Color = "#ff9e3b" } },
              { Foreground = { Color = "#111111" } },
              { Text = " COPY " },
              "ResetAttributes",
            }
          end
          table.insert(left, { Foreground = { Color = "#f0dfaf" } })
          table.insert(left, { Text = " " .. cmd .. " " })
          window:set_left_status(wezterm.format(left))
          window:set_right_status(wezterm.format({
            { Foreground = { Color = "#7f9f7f" } },
            { Text = " " .. host .. " " },
            { Foreground = { Color = "#dcdccc" } },
            { Text = cwd .. " " },
          }))
        end)

        return config
      '';
    };

    home.packages = [
      pkgs.nerd-fonts.jetbrains-mono
      pkgs.nerd-fonts.symbols-only
      # Fallback glyphs (⧉ ⑂ ★ ↻ ⟳ ⚑ ⎇ ▣ ⧈) the main font lacks
      pkgs.noto-fonts
    ];
  };
}
