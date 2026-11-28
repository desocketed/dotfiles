{ pkgs, config, lib, crane, ... }:
let
  cfg = config.modules.novamix;
  package = pkgs.callPackage ../../../pkgs/novamix { craneLib = crane.mkLib pkgs; };
in {
  options.modules.novamix.enable =
    lib.mkEnableOption "Arctis Nova Pro Wireless ChatMix: GAME and CHAT sinks driven by the base station's dial";

  options.modules.novamix.settings = lib.mkOption {
    type = (pkgs.formats.toml { }).type;
    default = { };
    description = ''
      Contents of ~/.config/novamix/config.toml: `[[route]]` rules sending
      streams to GAME or CHAT. Empty keeps the daemon's defaults.
    '';
  };

  config = lib.mkIf cfg.enable {
    # `novamix` (the CLI) beside the daemon the service runs
    home.packages = [ package ];

    # Only written when set: an empty file would replace the default rules.
    xdg.configFile."novamix/config.toml" = lib.mkIf (cfg.settings != { }) {
      source = (pkgs.formats.toml { }).generate "novamix.toml" cfg.settings;
    };

    systemd.user.services.novamix = {
      Unit = {
        Description = "Arctis Nova Pro Wireless ChatMix";
        PartOf = [ "pipewire.service" ];
        After = [ "pipewire.service" "wireplumber.service" ];
        Wants = [ "wireplumber.service" ];
      };
      Service = {
        ExecStart = lib.getExe package;
        Restart = "on-failure";
        RestartSec = 2;
        # pw-loopback and wpctl come from the host's PipeWire, not Nix's.
        Environment = [ "RUST_LOG=info" "PATH=/usr/local/bin:/usr/bin:/run/current-system/sw/bin:/etc/profiles/per-user/%u/bin" ];
      };
      Install.WantedBy = [ "default.target" ];
    };
  };
}
