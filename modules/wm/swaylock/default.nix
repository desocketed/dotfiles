{ config, lib, ... }:
let
  cfg = config.modules.swaylock;
in {
  options.modules.swaylock.enable = lib.mkEnableOption "swaylock";

  config = lib.mkIf cfg.enable {
    programs.swaylock = {
      enable = true;
      settings = {
        color = "000000";
        show-failed-attempts = true;
        indicator-caps-lock = true;
      };
    };
  };
}
