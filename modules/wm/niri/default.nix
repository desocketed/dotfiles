{ pkgs, config, lib, ... }:
let
  cfg = config.modules.niri;
  niri-shake-cursor = pkgs.stdenvNoCC.mkDerivation rec {
    pname = "niri-shake-cursor";
    version = "0.1.0";
    src = pkgs.fetchurl {
      url = "https://github.com/DavidePrette/niri-shake-cursor/releases/download/v${version}/niri-shake-cursor-v${version}-linux-amd64.tar.gz";
      hash = "sha256-DXoXIaqjG3/iZ6BGrWNsEn9KyiFiXZ2UmjYcIEiux6E=";
    };
    # Tarball has no top-level directory.
    sourceRoot = ".";
    installPhase = ''
      install -Dm755 niri-shake-cursor $out/bin/niri-shake-cursor
    '';
    meta = {
      description = "macOS-style shake-to-find-cursor for the niri compositor";
      homepage = "https://github.com/DavidePrette/niri-shake-cursor";
      license = lib.licenses.mit;
      platforms = [ "x86_64-linux" ];
      mainProgram = "niri-shake-cursor";
    };
  };
  liveFiles = [ "binds" "input" "layout" "misc" "rules" "workspaces" "local" ];
in {
  options.modules.niri = {
    enable = lib.mkEnableOption "the niri window manager";
    displays = lib.mkOption {
      type = import ../displays.nix { inherit lib; };
      default = [ ];
      description = "Physical outputs to configure.";
    };
  };

  imports = [
    ../fuzzel
    ../swaylock
  ];

  config = lib.mkIf cfg.enable {
    modules.fuzzel.enable = lib.mkDefault true;
    modules.swaylock.enable = lib.mkDefault true;

    programs.jq.enable = true;

    home.packages = [
      niri-shake-cursor
    ] ++ (with pkgs; [
      swaybg
      nirius
      wl-freeze
    ]);

    home.file = {
      # mine.kdl only holds the includes and stays a store file: niri resolves
      # them relative to where it was loaded from.
      "niri-mine" = {
        source = ./mine.kdl;
        target = ".config/niri/mine.kdl";
      };
      "niri-focus-or-switch-workspace" = {
        source = ./focus-or-switch-workspace.sh;
        target = ".config/niri/mine/focus-or-switch-workspace.sh";
        executable = true;
      };
      # Stable paths for the tools binds.kdl spawns, so the binds work without
      # ~/.nix-profile on PATH and binds.kdl needs no store paths.
      "niri-bin" = {
        source = pkgs.linkFarm "niri-bin" [
          { name = "emacs"; path = "${config.services.emacs.package}/bin/emacs"; }
          { name = "emacsclient"; path = "${config.services.emacs.package}/bin/emacsclient"; }
          { name = "mpc"; path = lib.getExe pkgs.mpc; }
          { name = "nu"; path = lib.getExe pkgs.nushell; }
          { name = "wl-freeze"; path = lib.getExe pkgs.wl-freeze; }
        ];
        target = ".config/niri/bin";
      };
      "niri-outputs" = {
        text = builtins.toString (
          map
            (display: ''
              output "${display.name}" {
                mode "${toString display.mode.width}x${toString display.mode.height}@${toString display.mode.rate}"
                scale ${toString display.scale}
                transform "${display.transform}"
                position x=${toString display.position.x} y=${toString display.position.y}
                ${if display.main or false then "focus-at-startup" else ""}
              }
            '')
            cfg.displays
        );
        target = ".config/niri/outputs.kdl";
      };
    }
    # Edited in place in the checkout, so changes apply without a rebuild.
    // builtins.listToAttrs (map (file: lib.nameValuePair "niri-${file}" {
      source = config.lib.file.mkOutOfStoreSymlink "${config.modules.dotfiles.dir}/modules/wm/niri/${file}.kdl";
      target = ".config/niri/mine/${file}.kdl";
    }) liveFiles);

    services.swayidle = {
      enable = true;
      timeouts = [{
        timeout = 300;
        command = "niri msg action power-off-monitors";
        resumeCommand = "niri msg action power-on-monitors";
      }];
    };

    # services.niri-autoselect-portal.enable = true;
  };
}
