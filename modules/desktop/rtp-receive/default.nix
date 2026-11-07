{ config, lib, ... }:
let
  cfg = config.modules.rtp-receive;
in {
  options.modules.rtp-receive.enable =
    lib.mkEnableOption "playing SAP-announced RTP audio streams from the LAN";

  # Listens for SAP announcements (224.0.0.56, 9875/udp) and plays each announced
  # RTP stream to the default sink.
  config = lib.mkIf cfg.enable {
    xdg.configFile."pipewire/pipewire.conf.d/60-rtp-receive.conf".text = ''
      context.modules = [
        {
          name = libpipewire-module-rtp-sap
          args = {
            sap.ip = 224.0.0.56
            sap.port = 9875
            stream.rules = [
              {
                matches = [ { rtp.session = "~.*" } ]
                actions = {
                  create-stream = {
                    sess.latency.msec = 30
                  }
                }
              }
            ]
          }
        }
      ]
    '';
  };
}
