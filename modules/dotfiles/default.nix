{ config, lib, ... }:
let
  cfg = config.modules.dotfiles;
in
{
  options.modules.dotfiles = {
    dir = lib.mkOption {
      type = lib.types.str;
      description = ''
        Absolute path of this checkout. Set it per machine in hosts/local.nix.
        Mutable files (Doom config, Niri config) are linked into it.
      '';
    };

    username = lib.mkOption {
      type = lib.types.str;
      description = "Login name of the user. Set it per machine in hosts/local.nix.";
    };

    homeDirectory = lib.mkOption {
      type = lib.types.str;
      default = "/home/${cfg.username}";
      defaultText = lib.literalExpression ''"/home/''${config.modules.dotfiles.username}"'';
      description = "Home directory of the user.";
    };

    timezone = lib.mkOption {
      type = lib.types.str;
      default = "UTC";
      description = "Timezone shown by the status bar.";
    };

    monitors = {
      main = lib.mkOption {
        type = lib.types.str;
        default = "HDMI-A-1";
        description = "Niri output name of the main external monitor.";
      };
      side = lib.mkOption {
        type = lib.types.str;
        default = "HDMI-A-2";
        description = "Niri output name of the secondary external monitor.";
      };
    };

    cachix = {
      name = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = null;
        description = "Cachix cache to pull from and push to; unset disables it.";
      };
      publicKey = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = null;
        description = "Public signing key of the Cachix cache, as shown by cachix.";
      };
    };
  };

  config.home = {
    username = cfg.username;
    homeDirectory = cfg.homeDirectory;
  };
}
