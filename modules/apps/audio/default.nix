{ config, lib, pkgs, ... }:
let
  cfg = config.modules.audio;
in {
  options.modules.audio = {
    enable = lib.mkEnableOption "PipeWire/WirePlumber config for the host's own audio stack, plus playerctld";
    lowLatency = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = "Allow PipeWire quanta down to 64 frames (about 1.3 ms at 48 kHz).";
    };
    noiseSuppression = {
      enable = lib.mkEnableOption "RNNoise noise suppression, exposed as a \"Mic (clean)\" source";
      source = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = null;
        description = "node.name of the microphone to clean; null follows the default source.";
      };
      vadThreshold = lib.mkOption {
        type = lib.types.ints.between 0 100;
        default = 50;
        description = "Voice activity threshold in percent; higher gates more aggressively.";
      };
    };
  };

  config = lib.mkIf cfg.enable {
    services.playerctld.enable = true;

    xdg.configFile."pipewire/pipewire.conf.d/80-low-latency.conf" = lib.mkIf cfg.lowLatency {
      text = ''
        context.properties = {
          default.clock.min-quantum = 64
        }
      '';
    };

    # The plugin is a plain LADSPA .so with no graphics, so the nixpkgs store
    # path is fine next to the host's own PipeWire (CachyOS's rnnoise package
    # ships only librnnoise, not the LADSPA wrapper PipeWire needs).
    xdg.configFile."pipewire/pipewire.conf.d/90-noise-suppression.conf" = lib.mkIf cfg.noiseSuppression.enable {
      text = ''
        context.modules = [
          {
            name = libpipewire-module-filter-chain
            args = {
              node.description = "Mic (clean)"
              media.name = "Mic (clean)"
              filter.graph = {
                nodes = [
                  {
                    type = ladspa
                    name = rnnoise
                    plugin = ${pkgs.rnnoise-plugin}/lib/ladspa/librnnoise_ladspa.so
                    label = noise_suppressor_mono
                    control = {
                      "VAD Threshold (%)" = ${toString cfg.noiseSuppression.vadThreshold}
                    }
                  }
                ]
              }
              audio.rate = 48000
              audio.position = [ MONO ]
              capture.props = {
                node.name = "capture.mic_clean"
                node.passive = true
        ${lib.optionalString (cfg.noiseSuppression.source != null) ''        target.object = "${cfg.noiseSuppression.source}"
        ''}      }
              playback.props = {
                node.name = "mic_clean"
                media.class = Audio/Source
              }
            }
          }
        ]
      '';
    };
  };
}
