{ config, lib, ... }:
let
  cfg = config.modules.notifications;
in {
  options.modules.notifications.enable = lib.mkEnableOption "the mako notification daemon";

  config = lib.mkIf cfg.enable {
    systemd.user.services.mako = {
      Unit = {
        PartOf = "graphical-session.target";
        After = "graphical-session.target";
        Requisite = "graphical-session.target";
      };
      Service = {
        ExecStart = lib.getExe config.services.mako.package;
        Restart = "on-failure";
      };
    };

    services.mako = {
      enable = true;
      settings = {
        default-timeout = 5000; # ms
        background-color = "#100116";
        border-color = "#5d077f";
        border-radius = 4;
      };
    };
  };
}
