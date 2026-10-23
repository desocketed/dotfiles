{ config, pkgs, lib, ... }:
let
  cfg = config.modules.terminal;
in {
  options.modules.terminal.enable = lib.mkEnableOption "foot and alacritty terminals";

  config = lib.mkIf cfg.enable {
    programs.foot = {
      enable = true;
      settings = {
        main = {
          shell = "zsh";
          font = "JetBrainsMono:size=12";
        };
        colors = {
          foreground = "dcdccc";
          background = "111111";
        };
      };
    };

    xdg.enable = true;
    programs.alacritty = {
      enable = true;
      settings = {
        terminal.shell = "${config.home.homeDirectory}/.nix-profile/bin/nu";
        font.normal.family = "JetBrains Mono";
        font.size = 12;
        # Chromeless: Zellij provides the tabs and panes
        window.decorations = "None";
        window.padding = { x = 0; y = 0; };
        keyboard.bindings = [
          { key = "Return"; mods = "Alt"; action = "SpawnNewInstance"; }
        ];
      };
    };

    home.packages = [ pkgs.nerd-fonts.jetbrains-mono ];
  };
}
