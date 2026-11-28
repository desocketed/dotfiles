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
      # DMS embeds its Qt runtime (dux-qt-base) and draws with GL, so the
      # whole shell needs the host's drivers through nixGL
      package = config.lib.nixGL.wrap (
        dms.packages.${pkgs.stdenv.hostPlatform.system}.dms-shell.overrideAttrs { doCheck = false; }
      );
    };

    # The desktop never locks: DMS's lock screen cannot authenticate against the
    # Nix PAM here, so turn off every way it locks itself (idle, suspend, logind
    # lock requests, fade, startup). Patches an existing settings.json only.
    home.activation.dmsNeverLock = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
      f="$HOME/.config/DankMaterialShell/settings.json"
      if [ -f "$f" ]; then
        tmp=$(mktemp)
        ${lib.getExe pkgs.jq} '. + {acLockTimeout: 0, batteryLockTimeout: 0, lockBeforeSuspend: false, loginctlLockIntegration: false, fadeToLockEnabled: false, lockAtStartup: false}' "$f" > "$tmp" && $DRY_RUN_CMD mv "$tmp" "$f" || rm -f "$tmp"
      fi
    '';

    # DMS runs its own notification server; mako would fight it for the bus name
    modules.notifications.enable = lib.mkForce false;
    modules.waybar.enable = lib.mkForce false;
  };
}
