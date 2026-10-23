{ config, lib, ... }:
let
  cfg = config.modules.wayland;
in {
  options.modules.wayland.enable = lib.mkEnableOption "Wayland session environment variables";

  config = lib.mkIf cfg.enable {
    home.sessionVariables = {
      MOZ_ENABLE_WAYLAND = 1;
      QT_QPA_PLATFORM = "wayland";
    };
  };
}
