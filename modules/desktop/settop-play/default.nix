{ config, lib, pkgs, ... }:
let
  cfg = config.modules.settop-play;
in {
  options.modules.settop-play = {
    enable = lib.mkEnableOption "`settop`: send a video URL to the set-top box over SSH";
    host = lib.mkOption {
      type = lib.types.str;
      default = "";
      description = "Address of the set-top box (set in hosts/local.nix).";
    };
    user = lib.mkOption {
      type = lib.types.str;
      default = "";
      description = "SSH user on the set-top box (set in hosts/local.nix).";
    };
    key = lib.mkOption {
      type = lib.types.str;
      default = "${config.home.homeDirectory}/.ssh/settop-play";
      description = "Private key the box accepts only for playback requests.";
    };
  };

  # The box forces this key to its settop-play-recv command, which reads one request line:
  # `play <url>`, `play-tv <url>` or `stop`.
  config = lib.mkIf cfg.enable {
    home.packages = [
      (pkgs.writeShellApplication {
        name = "settop";
        runtimeInputs = [ pkgs.openssh ];
        text = ''
          usage() { echo "usage: settop play [--tv] <url> | settop stop" >&2; exit 2; }
          host=${lib.escapeShellArg cfg.host}
          user=${lib.escapeShellArg cfg.user}
          if [ -z "$host" ] || [ -z "$user" ]; then
            echo "settop: set modules.settop-play.host and .user in hosts/local.nix" >&2
            exit 1
          fi
          case "''${1:-}" in
            play)
              shift
              req=play
              if [ "''${1:-}" = --tv ]; then req=play-tv; shift; fi
              [ $# -eq 1 ] || usage
              line="$req $1"
              ;;
            stop) [ $# -eq 1 ] || usage; line=stop ;;
            *) usage ;;
          esac
          printf '%s\n' "$line" | ssh -T -i ${lib.escapeShellArg cfg.key} \
            -o IdentitiesOnly=yes -o BatchMode=yes "$user@$host"
        '';
      })
    ];
  };
}
