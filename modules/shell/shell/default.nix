{ config, pkgs, lib, ... }:
let
  cfg = config.modules.shell;

  aliases = {
    e = "emacsclient -n";
    tmp = "cd $(mktemp -d)";
    tree = "lt";
    watch = "watch -n 1";
    dud = "du -h -d1";
    sd = "sudo systemctl";
    sdu = "systemctl --user";
    j = "journalctl -u";
    ju = "journalctl --user -u";
    open = "xdg-open";
    play = "mpv --hwdec=autosafe";
    dl = "yt-dlp -f bestvideo+bestaudio";
    up = "$HOME/sh/update.sh";
    ipython3 = "ipython3 --no-confirm-exit";
  };
in
{
  options.modules.shell.enable = lib.mkEnableOption "shell (zsh/nushell) and gpg-agent config";

  config = lib.mkIf cfg.enable {
    home.packages = with pkgs; [
      zsh-vi-mode
      fd
      agkozak-zsh-prompt
      gcr_4
      nufmt
    ];

    programs.fzf.enable = true;

    programs.gpg.enable = true;
    services.gpg-agent = {
      enable = true;
      enableSshSupport = true;
      pinentry.package = pkgs.pinentry-gnome3;
      defaultCacheTtl = 60480000;
      maxCacheTtl = 60480000;
      defaultCacheTtlSsh = 60480000;
      maxCacheTtlSsh = 60480000;
      grabKeyboardAndMouse = true;
      extraConfig = ''
        allow-emacs-pinentry
        allow-loopback-pinentry
      '';
    };

    # Exported by hm-session-vars.sh, which zsh sources from .zshenv.
    home.sessionVariables = {
      ALTERNATE_EDITOR = "";
      EDITOR = "emacsclient -c";
      VISUAL = "emacsclient -c -a emacs";
      TERM = "xterm";
      SSH_AGENT_PID = "";
      SSH_AUTH_SOCK = "$XDG_RUNTIME_DIR/gnupg/S.gpg-agent.ssh";
      CARGO_TARGET_DIR = "$HOME/.cache/cargo/target";

      # This fixes games taking a long time to launch with Steam. I
      # tried setting this to "1", but that didn't seem to work. Not
      # sure what other values are valid
      STEAMOS = "ANYTHING";
    };

    # Zsh
    programs.fzf.enableZshIntegration = true;
    services.gpg-agent.enableZshIntegration = true;
    programs.zsh = {
      enable = true;
      autosuggestion.enable = true;
      syntaxHighlighting.enable = true;
      history.save = 100000;
      history.size = 100000;
      history.share = true;
      shellAliases = aliases;
      initContent = ''
        alias cat=bat
        if [ -e ${config.home.homeDirectory}/.nix-profile/etc/profile.d/nix.sh ] ; then
          source ${config.home.homeDirectory}/.nix-profile/etc/profile.d/nix.sh
        fi

        export PATH="$PATH:$HOME/.local/bin";

        alias ..="cd .."
        alias ...="cd ../.."

        [[ -f "/tmp/emacs-share-path" ]] \
          && [[ -d "$(cat /tmp/emacs-share-path)" ]] \
          && cd "$(/bin/cat /tmp/emacs-share-path)"

        export PATH=$HOME/.cargo/bin:$PATH

        [[ -f "$HOME/.cargo/env" ]] \
          && source "$HOME/.cargo/env"

        bindkey "^[[1;5C" forward-word
        bindkey "^[[1;5D" backward-word

        source ${pkgs.zsh-vi-mode}/share/zsh-vi-mode/zsh-vi-mode.plugin.zsh
        source ${pkgs.agkozak-zsh-prompt}/share/zsh/site-functions/agkozak-zsh-prompt.plugin.zsh
      '';

      # 2026-01-27
      # warning: The default value of `programs.zsh.dotDir` will change in future versions.
      # You are currently using the legacy default (home directory) because `home.stateVersion` is less than "26.05".
      # To silence this warning and lock in the current behavior, set:
      #   programs.zsh.dotDir = config.home.homeDirectory;
      # To adopt the new behavior (XDG config directory), set:
      #   programs.zsh.dotDir = "${config.xdg.configHome}/zsh";
      dotDir = "${config.xdg.configHome}/zsh";
    };

    # Nushell
    services.gpg-agent.enableNushellIntegration = true;
    programs.nushell = {
      enable = true;
      extraConfig = ''
        $env.PATH = ($env.PATH | prepend [ "~/.nix-profile/bin" "~/.cargo/bin" ])
        $env.CARGO_TARGET_DIR = echo $env.HOME ".cache/cargo/target" | path join
        $env.DOTNET_CLI_TELEMETRY_OPTOUT = 1
      '';

      settings = {
        show_banner = false;
        table.mode = "light";
        completions.algorithm ="fuzzy";
        buffer_editor = [
            "emacsclient"
            "-n"
            "-r"
        ];
        completions.external = {
          enable = true;
        };
      };
    };

    programs.bat.enable = true;

    programs.eza = {
      enable = true;
      icons = "auto";
    };
    programs.zoxide = {
      enable = true;
      enableNushellIntegration = true;
    };
  };
}
