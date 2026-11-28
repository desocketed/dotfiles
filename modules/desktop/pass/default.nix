{ pkgs, config, lib, ... }:
let
  cfg = config.modules.pass;
in {
  options.modules.pass.enable = lib.mkEnableOption "the pass password store";

  config = lib.mkIf cfg.enable {
    home.packages = with pkgs; [
      passExtensions.pass-otp
      wl-clipboard
    ];

    programs.password-store = {
      enable = true;
      settings = {
        PASSWORD_STORE_DIR = "$HOME/.password-store";
      };
    };
  };
}
