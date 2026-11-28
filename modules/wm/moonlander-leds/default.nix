{ pkgs, config, lib, crane, qmk_firmware, ... }:
let
  cfg = config.modules.moonlander-leds;
  package = pkgs.callPackage ../../../pkgs/moonlander-leds { craneLib = crane.mkLib pkgs; qmkSrc = qmk_firmware; };
in {
  options.modules.moonlander-leds.enable =
    lib.mkEnableOption "Moonlander LEDs driven by desktop state (Niri workspaces, MPRIS) over raw HID";

  options.modules.moonlander-leds.settings = lib.mkOption {
    type = (pkgs.formats.toml { }).type;
    default = { };
    description = ''
      Contents of ~/.config/moonlander-leds/config.toml. Keys: empty,
      occupied, focused, urgent, visible (colours as "#rrggbb") and dim
      (brightness of an unfocused display, 0.0 to 1.0); for the media keys
      media_playing, media_paused, media_none (play/pause key) and
      media_skip, media_noskip (previous/next key, when the player can or
      cannot go that way). Unset keys keep the daemon's defaults.
    '';
  };

  config = lib.mkIf cfg.enable {
    # The control commands (set, get, reset, status) talk to the service.
    home.packages = [ package ];

    xdg.configFile."moonlander-leds/config.toml".source =
      (pkgs.formats.toml { }).generate "moonlander-leds.toml" cfg.settings;

    systemd.user.services.moonlander-leds = {
      Unit = {
        Description = "Moonlander LEDs from desktop state";
        PartOf = [ "graphical-session.target" ];
        After = [ "graphical-session.target" ];
      };
      Service = {
        ExecStart = lib.getExe package;
        Restart = "on-failure";
        RestartSec = 2;
        # niri comes from the host (/usr/local/bin on the desktop).
        Environment = [ "RUST_LOG=info" "PATH=%h/.nix-profile/bin:/usr/local/bin:/usr/bin:/run/current-system/sw/bin:/etc/profiles/per-user/%u/bin" ];
      };
      Install.WantedBy = [ "graphical-session.target" ];
    };
  };
}
