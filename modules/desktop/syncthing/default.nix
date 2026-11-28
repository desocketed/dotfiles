{ config, lib, ... }:
let
  cfg = config.modules.syncthing;
in {
  options.modules.syncthing.enable = lib.mkEnableOption "Syncthing";

  config = lib.mkIf cfg.enable {
    services.syncthing.enable = true;
  };
}
