{ config, lib, pkgs, dms, ... }:
let
  cfg = config.modules.dms;
in {
  options.modules.dms.enable = lib.mkEnableOption "DankMaterialShell, the bar, launcher and notifications in one";

  imports = [ dms.homeModules.default ];

  config = lib.mkIf cfg.enable {
    programs.dank-material-shell = {
      enable = true;
      # Started with the graphical session, like mako and waybar
      systemd.enable = true;
      enableCalendarEvents = true;
      # Its Go tests bind a unix socket under the build dir, which is longer
      # than a socket path may be in the sandbox
      # Quickshell draws with GL, so it needs the host's drivers through nixGL
      quickshell.package = config.lib.nixGL.wrap pkgs.quickshell;
      package = dms.packages.${pkgs.stdenv.hostPlatform.system}.dms-shell.overrideAttrs { doCheck = false; };
    };

    # DMS runs its own notification server; mako would fight it for the bus name
    modules.notifications.enable = lib.mkForce false;
    modules.waybar.enable = lib.mkForce false;
  };
}
