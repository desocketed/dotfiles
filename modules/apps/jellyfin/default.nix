{ pkgs, config, lib, ... }:
let
  cfg = config.modules.jellyfin;
in {
  options.modules.jellyfin.enable = lib.mkEnableOption "the Jellyfin client";

  config = lib.mkIf cfg.enable {
    home.packages = with pkgs; [ jellyfin ];

    systemd.user.services.jellyfin = {
      Unit = { After = "graphical.target"; };
      Service = {
        ExecStart = lib.getExe pkgs.jellyfin;
      };
      Install = { WantedBy = [ "default.target" ]; };
    };
  };
}
