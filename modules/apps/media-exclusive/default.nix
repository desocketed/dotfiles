{ config, lib, pkgs, ... }:
let
  cfg = config.modules.media-exclusive;
  # Reads "<instance> <status>" lines; when one player starts, pause the rest.
  script = pkgs.writeShellScript "media-exclusive" ''
    ${lib.getExe pkgs.playerctl} --all-players --follow status --format '{{playerInstance}} {{status}}' |
      while read -r player status; do
        [ "$status" = Playing ] && ${lib.getExe pkgs.playerctl} --all-players --ignore-player "$player" pause
      done
  '';
in {
  options.modules.media-exclusive.enable =
    lib.mkEnableOption "pausing every other MPRIS player when one starts playing";

  config = lib.mkIf cfg.enable {
    services.playerctld.enable = true;

    systemd.user.services.media-exclusive = {
      Unit = {
        Description = "Pause other MPRIS players when one starts playing";
        After = [ "graphical-session.target" ];
        PartOf = [ "graphical-session.target" ];
      };
      Service = {
        ExecStart = script;
        Restart = "on-failure";
        RestartSec = 2;
      };
      Install.WantedBy = [ "graphical-session.target" ];
    };
  };
}
