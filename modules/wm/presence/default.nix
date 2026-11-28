{ pkgs, config, lib, ... }:
let
  cfg = config.modules.presence;
  # presence-set <state>: atomically write state, and `since` when it changed.
  set = pkgs.writeShellApplication {
    name = "presence-set";
    runtimeInputs = [ pkgs.coreutils ];
    text = ''
      dir=${lib.escapeShellArg cfg.dir}
      mkdir -p "$dir"
      old=$(cat "$dir/state" 2>/dev/null || true)
      [ "$old" = "$1" ] && exit 0
      date +%s > "$dir/since.tmp" && mv "$dir/since.tmp" "$dir/since"
      echo "$1" > "$dir/state.tmp" && mv "$dir/state.tmp" "$dir/state"
    '';
  };
  setCmd = state: "${lib.getExe set} ${state}";
in {
  options.modules.presence = {
    enable = lib.mkEnableOption "presence signal (desktop|away) for the assistant container, from the session's swayidle";
    dir = lib.mkOption {
      type = lib.types.str;
      default = "${config.home.homeDirectory}/claude/.services/presence";
      description = "Directory receiving the `state` and `since` files.";
    };
    idleSeconds = lib.mkOption {
      type = lib.types.int;
      default = 300;
      description = "Seconds without input before the desktop counts as away.";
    };
  };

  # Rides on the swayidle the niri module already runs: one more timeout, and the unit's start/stop mark the session.
  config = lib.mkIf cfg.enable {
    services.swayidle.timeouts = [{
      timeout = cfg.idleSeconds;
      command = setCmd "away";
      resumeCommand = setCmd "desktop";
    }];
    systemd.user.services.swayidle.Service = {
      ExecStartPost = setCmd "desktop";
      ExecStopPost = setCmd "away";
    };
  };
}
