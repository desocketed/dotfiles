{ config, pkgs, ... }:
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
    # The desktop never locks: no swaylock, no lock bind (see the dms module too)
    swaylock.enable = false;
    dms.enable = true;
    moonlander-leds.enable = true;
    novamix = {
      enable = true;
      settings = {
        take_default = true;
        sinks = {
          game = "MEDIA";
          chat = "CHAT";
          global = "GLOBAL";
        };
      };
    };
    claude-snap.enable = true;

    audio.enable = true;
    discord.enable = true;
    jellyfin.enable = true;
    lubelogger.enable = false;
    music.enable = true;
    media-exclusive.enable = true;
    presence.enable = true;
    rtp-receive.enable = true;
    settop-play.enable = true;
    syncthing.enable = true;
    wayland.enable = true;

    dev.enable = true;
    emacs.enable = true;
    shell.enable = true;
    terminal.enable = true;
    tools.enable = true;
  };

  home.packages = [ pkgs.qmk ];

  programs.homun = {
    enable = true;
    cpuset = "12-15,28-31";
    kvm = true;
    # the cockpit stays a Doom build from this checkout (it is visible in the containers)
    cockpit.config = "${config.modules.dotfiles.dir}/doom";
    ports = [
      { host = 8080; container = 8080; scope = "lan"; }
      { host = 8443; container = 8443; scope = "lan"; }
    ];
  };

  # When the default sink is missing (a missing default sink), WirePlumber
  # falls back to the sink with the highest priority: make that the headset, not HDMI or onboard
  xdg.configFile."wireplumber/wireplumber.conf.d/51-headset-first.conf".text = ''
    monitor.alsa.rules = [
      {
        matches = [ { node.name = "~alsa_output.*SteelSeries_Arctis_Nova_Pro_Wireless.*" } ]
        actions = { update-props = { priority.session = 2000 } }
      }
    ]
  '';
}
