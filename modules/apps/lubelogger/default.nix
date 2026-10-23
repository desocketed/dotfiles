{ pkgs, config, lib, ... }:
let
  cfg = config.modules.lubelogger;
in {
  options.modules.lubelogger.enable = lib.mkEnableOption "the LubeLogger container";

  config = lib.mkIf cfg.enable {
    services.podman = {
      enable = true;
      containers = {
        "lubelogger" = {
          image = "ghcr.io/hargata/lubelogger";
          ports = [ "8080:8080" ];
          autoStart = true;
          autoUpdate = "local";
          volumes = [
            "${config.home.homeDirectory}/apps/lubelogger/data:/App/data"
            "${config.home.homeDirectory}/apps/lubelogger/keys:/root/.aspnet/DataProtection-Keys"
          ];
          environment = {
            LC_ALL = "en_US.UTF-8";
            LANG = "en_US.UTF-8";
          };
        };
      };
    };
  };
}
