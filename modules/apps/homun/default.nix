{ config, lib, pkgs, homun, ... }:
let
  cfg = config.programs.homun;
  home = config.home.homeDirectory;

  # The wrapper reads this one file for everything specific to the machine
  # (lib/hostcfg.nu in the wrapper); unset options are left out.
  hostConfig = lib.filterAttrs (_: v: v != null && v != [ ] && v != false) {
    inherit (cfg) user settop cpuset gpu kvm roles;
    report = if cfg.reportRepo == null then null else { repo = cfg.reportRepo; };
    cockpit = if cfg.cockpit.config == null then null else { inherit (cfg.cockpit) config; };
    ports = map (p: lib.filterAttrs (_: v: v != null) p) cfg.ports;
  };

  # What the host service runs with; podman and systemd come from the host.
  toolPath = lib.concatStringsSep ":" (
    map (p: "${p}/bin") (with pkgs; [ nushell zellij just git chafa imagemagick xdg-dbus-proxy ])
    ++ [ "${home}/.local/bin" "${home}/.nix-profile/bin" "/run/current-system/sw/bin" "/run/wrappers/bin" "/usr/local/bin" "/usr/bin" "/bin" ]
  );

  url = "https://github.com/dsktd/homun.git";

  # Before each start of the host service: clone on a first switch, and bring a clone from
  # before the service existed to the pin once (bootstrap.sh)
  bootstrap = pkgs.writeShellScript "homun-bootstrap" (builtins.readFile ./bootstrap.sh);

  portType = lib.types.submodule {
    options = {
      host = lib.mkOption { type = lib.types.port; description = "Port on the host."; };
      container = lib.mkOption { type = lib.types.port; description = "Port in the container."; };
      proto = lib.mkOption { type = lib.types.enum [ "tcp" "udp" ]; default = "tcp"; };
      from = lib.mkOption {
        type = lib.types.enum [ "container" "host" ];
        default = "container";
        description = "container: the host port reaches the container's; host: the container reaches the host's.";
      };
      scope = lib.mkOption {
        type = lib.types.enum [ "host" "lan" ];
        default = "host";
        description = "host: listen on 127.0.0.1; lan: on every IPv4 interface (put something that authenticates in front).";
      };
    };
  };
in
{
  options.programs.homun = {
    enable = lib.mkEnableOption "the host side of the Homun launcher (the launcher): `homun` on PATH, its config file and its host service";

    dir = lib.mkOption {
      type = lib.types.str;
      default = "${home}/.local/share/homun";
      example = "${home}/claude/_tools/homun";
      description = "Directory the launcher runs from: a git clone the host service makes when it is missing. The service checks it out at the commit the flake lock pins (input `homun`) while it has no local changes or commits, then builds the image for that commit and recreates the containers, all in the background (`homun host show` says how far it is). Or point at a checkout to run live: its own commits are left alone.";
    };
    user = lib.mkOption {
      type = lib.types.str;
      default = config.home.username;
      description = "The host user the containers act for.";
    };
    settop = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      description = "user@host the screensaver's settop poll connects to; null disables the poll.";
    };
    reportRepo = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      example = "owner/inbox";
      description = "Private repository (owner/name) `homun report` files bug reports into; null leaves reporting off.";
    };
    cpuset = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      example = "12-15,28-31";
      description = "CPUs the Manager's container is pinned to; null keeps the launcher's default.";
    };
    gpu = lib.mkEnableOption "/dev/dri inside the Manager's container";
    kvm = lib.mkEnableOption "/dev/kvm inside the Manager's container (accelerated emulators)";
    roles = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ];
      example = [ "mechanic" ];
      description = "Side roles whose containers always start with the Manager's.";
    };
    ports = lib.mkOption {
      type = lib.types.listOf portType;
      default = [ ];
      description = "Port relays that are always open, next to those added with `homun port add`.";
    };
    cockpit.config = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      example = "/home/me/.config/doom";
      description = "Directory of your own Emacs configuration for the cockpit, loaded over its plain one; a Doom directory makes the cockpit a Doom build. null keeps the cockpit as it is (plain on a new host).";
    };
    units = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Enable the host service homun-host, which answers the containers' requests, keeps them up and follows the pin (`homun host systemd add` otherwise, which never moves the clone).";
    };
  };

  config = lib.mkIf cfg.enable {
    # the launcher builds its image with just on first start
    home.packages = [ pkgs.just ];

    # Same as `just install`: a link to the launcher
    home.file.".local/bin/homun".source = config.lib.file.mkOutOfStoreSymlink "${cfg.dir}/run.nu";

    # The pin: the commit of the flake lock's input `homun`. The host service brings the clone to
    # it (only while the clone holds no local changes or commits), then builds the image and rolls
    # the containers, all in the background.
    xdg.configFile."homun/pin".text = builtins.toJSON { inherit (homun) rev; inherit url; };

    # A switch only tells the service to look again; `homun host show` says how far it is
    home.activation.restartHomunHost = lib.mkIf cfg.units (lib.hm.dag.entryAfter [ "reloadSystemd" ] ''
      run ${config.systemd.user.systemctlPath} --user restart --no-block homun-host.service \
        || echo "warning: could not restart homun-host.service; see: systemctl --user status homun-host" >&2
    '');

    xdg.configFile."claude/config.json".text = builtins.toJSON hostConfig;

    # The one host process next to the containers: it answers their requests, keeps them up and
    # follows the pin (bin/homun-host in the launcher)
    systemd.user.services.homun-host = lib.mkIf cfg.units {
      Unit.Description = "Homun host service: requests from the containers, supervision, updates";
      Service = {
        Type = "simple";
        # Containers started from here must outlive a restart of the service
        KillMode = "process";
        WorkingDirectory = home;
        Environment = [
          "PATH=${toolPath}"
          "GIT_SSL_CAINFO=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
        ];
        ExecStartPre = "${bootstrap} ${cfg.dir} ${homun.rev} ${url}";
        ExecStart = "${cfg.dir}/run.nu host run";
        Restart = "always";
        RestartSec = 30;
      };
      Install.WantedBy = [ "default.target" ];
    };
  };
}
