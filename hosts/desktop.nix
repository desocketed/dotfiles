{ ... }:
let
  center_display = "DP-1";
  right_display = "DP-2";

  displays = [
    {
      name = center_display;
      main = true;
      bg = "#100116";
      mode = {
        width = 2560;
        height = 1440;
        rate = 164.998;
      };
      position = {
        x = 0;
        y = 570;
      };
      transform = "normal";
      scale = 1.0;
    }
    {
      name = right_display;
      bg = "#000000";
      mode = {
        width = 2560;
        height = 1440;
        rate = 59.951;
      };
      position = {
        x = 2560;
        y = 0;
      };
      transform = "270";
      scale = 1.0;
    }
  ];
in
{
  modules = {
    niri = {
      enable = true;
      displays = displays;
    };
    notifications.enable = true;

    discord.enable = true;
    jellyfin.enable = true;
    lubelogger.enable = false;
    music.enable = true;
    syncthing.enable = true;
    wayland.enable = true;

    dev.enable = true;
    emacs.enable = true;
    shell.enable = true;
    terminal.enable = true;
    tools.enable = true;
  };
}
