# Framework laptop with the two BenQ monitors: shared by the framework and work hosts.
{ config, ... }:
let
  monitors = config.modules.dotfiles.monitors;
  displays = [
    {
      name = "eDP-1";
      mode = {
        width = 2880;
        height = 1920;
        rate = 120.000;
      };
      position = {
        x = 0;
        y = 570;
      };
      transform = "normal";
      scale = 1.0;
    }
    {
      name = monitors.main;
      main = true;
      mode = {
        width = 2560;
        height = 1440;
        rate = 239.999;
      };
      position = {
        x = 2880;
        y = 570;
      };
      transform = "normal";
      scale = 1.0;
    }
    {
      name = monitors.side;
      mode = {
        width = 2560;
        height = 1440;
        rate = 239.999;
      };
      position = {
        x = 5440;
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

    wayland.enable = true;

    dev.enable = true;
    emacs.enable = true;
    shell.enable = true;
    terminal.enable = true;
    tools.enable = true;
  };

  services.poweralertd.enable = true;
}
