{ pkgs, config, lib, ... }:
let
  cfg = config.modules.music;
in {
  options.modules.music.enable = lib.mkEnableOption "MPD";

  config = lib.mkIf cfg.enable {
    services.mpd = {
      enable = true;
      network.startWhenNeeded = true;
      musicDirectory = "${config.home.homeDirectory}/Music";
      extraConfig = ''
        audio_output {
            type            "pipewire"
            name            "PipeWire Sound Server"
        }
      '';
    };

    home.packages = with pkgs; [
      mpc
    ];
  };
}
