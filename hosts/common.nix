{
  nixpkgs,
  config,
  pkgs,
  nur,
  emacs-overlay,
  lib,
  ...
}:
let
  cachix = config.modules.dotfiles.cachix;
in
{
  nixpkgs.overlays = [
    nur.overlays.default
    emacs-overlay.overlays.default
  ];

  imports = [
    ../modules/dotfiles
    ../modules/wm
    ../modules/apps
    ../modules/desktop
    ../modules/shell
  ]
  ++ lib.optional (builtins.pathExists ./local.nix) ./local.nix;

  nix = {
    package = pkgs.nix;
    settings = {
      experimental-features = [
        "nix-command"
        "flakes"
      ];
      max-jobs = "auto";
      cores = 0;
      extra-substituters = lib.optional (cachix.name != null) "https://${cachix.name}.cachix.org";
      extra-trusted-public-keys = lib.optional (
        cachix.name != null && cachix.publicKey != null
      ) "${cachix.name}.cachix.org-1:${cachix.publicKey}";
      post-build-hook = "${config.home.homeDirectory}/.config/nix/post-build-hook.sh";
    };
  };

  home.file.".config/nix/post-build-hook.sh" = {
    executable = true;
    text = ''
      #!/bin/sh
      # Optional: skip without a token, and never fail the build on a push error
      set -u
      set -f
      token="$HOME/.config/nix/cachix-auth-token"
      [ -r "$token" ] || exit 0
      [ -n ${lib.escapeShellArg (toString cachix.name)} ] || exit 0
      CACHIX_AUTH_TOKEN="$(cat "$token")" \
        ${pkgs.cachix}/bin/cachix push ${lib.escapeShellArg (toString cachix.name)} $OUT_PATHS \
        || echo "post-build-hook: cachix push failed, continuing" >&2
    '';
  };

  # PAM/systemd reads ~/.config/environment.d/*.conf directly at session
  # start, before any shell runs, so this applies to the compositor and
  # everything it spawns (launchers, D-Bus-activated apps, systemd user
  # units) even when the login shell (sh) and the interactive shell set in
  # the terminal emulator both skip sourcing hm-session-vars.sh.
  systemd.user.sessionVariables = {
    PATH = "$HOME/.nix-profile/bin:$HOME/.cargo/bin:$PATH:$HOME/.local/bin";
  };

  home.stateVersion = "24.05";
  programs.home-manager.enable = true;
}
