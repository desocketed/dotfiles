{ pkgs, config, lib, ... }:
let
  cfg = config.modules.music;
  musicDir = "${config.home.homeDirectory}/Music";
  # Queue the streams of a .pls file (File<n>=<url> lines) once mpd answers; no database or playlist dir involved.
  loadPlaylist = pkgs.writeShellApplication {
    name = "mpc-load-pls";
    runtimeInputs = [ pkgs.mpc pkgs.gnused pkgs.coreutils ];
    text = ''
      pls=$1
      for _ in $(seq 30); do mpc -q status >/dev/null 2>&1 && break; sleep 1; done
      mpc -q clear
      sed -n 's/^File[0-9]*=//p' "$pls" | tr -d '\r' | mpc -q add
    '';
  };
in {
  options.modules.music = {
    enable = lib.mkEnableOption "MPD";
    playlist = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      description = "A .pls file under ~/Music that the mpc service queues after mpd starts.";
    };
  };

  config = lib.mkIf cfg.enable {
    services.mpd = {
      enable = true;
      musicDirectory = musicDir;
      extraConfig = ''
        audio_output {
            type            "pipewire"
            name            "PipeWire Sound Server"
        }
      '';
    };
    # MPRIS bridge: mpd shows up to playerctl, media keys and the desktop's player widgets
    services.mpd-mpris.enable = true;
    # The PipeWire output needs the user's PipeWire up first.
    systemd.user.services.mpd.Unit = {
      After = [ "pipewire.service" ];
      Wants = [ "pipewire.service" ];
    };

    systemd.user.services.mpc = lib.mkIf (cfg.playlist != null) {
      Unit = {
        Description = "Queue ${cfg.playlist} in mpd";
        After = [ "mpd.service" ];
        Requires = [ "mpd.service" ];
      };
      Service = {
        Type = "oneshot";
        RemainAfterExit = true;
        ExecStart = "${lib.getExe loadPlaylist} ${musicDir}/${cfg.playlist}";
      };
      Install.WantedBy = [ "default.target" ];
    };

    home.packages = [ pkgs.mpc ];
  };
}
