{ config, lib, pkgs, ... }:
let
  cfg = config.modules.claude-snap;
  just = "${pkgs.just}/bin/just -f ${config.home.homeDirectory}/claude/_tools/homun/justfile";
in
{
  options.modules.claude-snap.enable =
    lib.mkEnableOption "hourly snapshots of ~/claude/.bare and .services, pruned to the newest 48 (plus 10 pre-recreate)";

  config = lib.mkIf cfg.enable {
    # `just snap` and `snap-prune` run `sudo -n bcachefs subvolume ...`: needs the sudoers rule
    systemd.user.services.claude-snap = {
      Unit.Description = "Snapshot ~/claude/.bare and .services, then prune";
      Service = {
        Type = "oneshot";
        Environment = [ "PATH=/usr/bin:/run/wrappers/bin" ];
        ExecStart = "${just} snap";
        ExecStartPost = "${just} snap-prune";
      };
    };
    systemd.user.timers.claude-snap = {
      Unit.Description = "Hourly claude-snap";
      Timer = {
        OnCalendar = "hourly";
        Persistent = true;
      };
      Install.WantedBy = [ "timers.target" ];
    };
  };
}
