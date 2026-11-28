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
    # mpv resumes where it was quit
    xdg.configFile."mpv/mpv.conf".text = ''
      save-position-on-quit=yes
    '';

    home.packages = with pkgs; [
      zsh-vi-mode
      fd
      agkozak-zsh-prompt
      gcr_4
      nufmt
    ];

    programs.fzf.enable = true;
    # fzf 0.74's completion.nu still uses the positional `|spans|` completer that
    # Nushell 0.116 deprecates (a warning on every prompt); zsh keeps fzf, nu uses
    # its own fuzzy completion
    programs.fzf.enableNushellIntegration = false;

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
      # Keep the legacy default until home.stateVersion moves to 26.11
      loadSessionVariables = false;
      extraConfig = ''
        $env.PATH = ($env.PATH | prepend [ "~/.nix-profile/bin" "~/.cargo/bin" ])
        $env.CARGO_TARGET_DIR = echo $env.HOME ".cache/cargo/target" | path join
        $env.DOTNET_CLI_TELEMETRY_OPTOUT = 1
        # Prompt: HH:MM:SS then the directory on the left, nothing on the right
        $env.PROMPT_COMMAND = {||
          let dir = (pwd | str replace $nu.home-dir "~")
          $"(ansi dark_gray)(date now | format date '%H:%M:%S')(ansi reset) (ansi green_bold)($dir)(ansi reset)"
        }
        $env.PROMPT_COMMAND_RIGHT = ""

        # nix: build/run/develop/shell/flake check also log to the buildlog service
        def --wrapped nix [...args: string] {
          let words = $args | where {|a| not ($a | str starts-with "-") } | first 2
          if not (($words | get 0? | default "") in [develop build run shell] or $words == [flake check]) {
            return (^nix ...$args)
          }
          # an interactive shell is not a build: no logging
          if ($words | get 0) == "develop" and not ($args | any {|a| $a in ["-c" "--command"] }) {
            return (^nix ...$args)
          }
          let dir = $nu.home-dir | path join claude .services buildlog nix
          mkdir $dir
          let log = $dir | path join $"(date now | date to-timezone UTC | format date '%Y%m%dT%H%M%SZ').log"
          let cmd = ["nix" ...$args] | each {|a| $"'($a | str replace -a "'" "'\\'''")'" } | str join " "
          let start = date now
          # script keeps nix on a pty, so it still sees a terminal (progress bars, colours)
          # sh hands script's exit code over in a file, so Nushell does not abort on a failed build
          ^sh -c 'script -q -e -f -c "$1" "$2"; echo $? > "$2.rc"' sh $cmd $log
          let code = open $"($log).rc" | into int
          rm $"($log).rc"
          let warnings = open $log | ansi strip | str replace -a "\r" "" | lines | where $it =~ "warning:" and $it !~ "^Script " | uniq
          { command: $"nix ($args | str join ' ')", cwd: (pwd), exit: $code, duration: ((date now) - $start | into int | $in / 1e9), log: $log, warnings: $warnings }
            | save -f ($dir | path join latest.json)
          ls $dir | where name =~ '\.log$' | sort-by name -r | skip 20 | each {|f| rm $f.name }
          if $code != 0 { ^sh -c $"exit ($code)" }
        }
      '';

      settings = {
        show_banner = false;
        # Kitty keyboard protocol: tells apart keys like S-RET and C-i
        use_kitty_protocol = true;
        keybindings = [
          {
            name = "newline_shift_enter";
            modifier = "shift";
            keycode = "enter";
            mode = [ "emacs" "vi_insert" ];
            event = { edit = "insertnewline"; };
          }
        ];
        table.mode = "light";
        completions.algorithm ="fuzzy";
        # Waits for the edit (C-x # in Emacs), so Nushell reads the buffer back;
        # -n returned at once and the edit was lost
        buffer_editor = [ "emacsclient" "-r" ];
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
