{ pkgs, config, lib, ... }:
let
  cfg = config.modules.dev;
in {
  options.modules.dev.enable = lib.mkEnableOption "dev tooling (direnv, git, language servers)";

  config = lib.mkIf cfg.enable {
    programs = {
      direnv.enable = true;
    };

    home.file.".config/rustfmt/rustfmt.toml".source = ./rustfmt.toml;

    programs.git = {
      enable = true;
      ignores = [ "*~" ];
      lfs.enable = true;
      settings = {
        # Some of this comes from:
        # https://blog.gitbutler.com/how-git-core-devs-configure-git/
        init = {
          defaultBranch = "main";
        };
        column = {
          ui = "auto";
        };
        branch = {
          sort = "-committerdate";
        };
        tag = {
          sort = "version:refname";
        };
        push = {
          autoSetupRemote = true;
          followTags = true;
        };
        fetch = {
          prune = true;
          pruneTags = true;
          all = true;
        };
        diff = {
          algorithm = "histogram";
          colorMoved = "plain";
          mnemonicPrefix = true;
          renames = true;
        };
        rerere = {
          enabled = true;
          autoupdate = true;
        };
        rebase = {
          autoSquash = true;
          autoStash = true;
          updateRefs = true;
        };
        merge = {
          conflictStyle = "zdiff3";
        };
        pull = {
          rebase = true;
        };
      };
    };

    programs.delta = {
      enable = true;
      enableGitIntegration = true;
    };

    home.packages = with pkgs; [
      nil
      nixpkgs-fmt
      cachix
      bash-language-server
      pyright
      ruff
      shellcheck
      just
      hut
      # eask-cli
      rye
      libz
      hugo
      sfz
      taplo
      lurk
      netcat
      dotnet-sdk
      nuget
      shfmt
      gopls
      zig
    ];
  };
}
