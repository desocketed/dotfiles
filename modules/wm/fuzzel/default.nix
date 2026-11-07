{ config, lib, ... }:
let
  cfg = config.modules.fuzzel;
in {
  options.modules.fuzzel.enable = lib.mkEnableOption "the fuzzel application launcher";

  config = lib.mkIf cfg.enable {
    programs.fuzzel = {
      enable = true;
      settings = {
        main = {
          layer = "overlay";
        };
        key-bindings = {
          # delete-line = "none";
          delete-line-forward = "none";
          prev = "Up Control+k";
          next = "Down Control+j";
        };
      };
    };
  };
}
