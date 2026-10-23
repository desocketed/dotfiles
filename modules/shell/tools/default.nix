{ pkgs, config, lib, ... }:
let
  cfg = config.modules.tools;
in {
  options.modules.tools.enable = lib.mkEnableOption "misc CLI tools (paru, zathura, ripgrep, etc.)";

  config = lib.mkIf cfg.enable {
    programs.zathura.enable = true;

    home.file.".config/paru/paru.conf".source = ./paru.conf;

    home.packages = with pkgs; [
      mtr
      procs
      ripgrep
      bottom
      btop
      rsync
      gping
      fzf
      uv
      bfs
      watchexec
      ledger
      hledger
      topgrade
      jnv
      impala
      nixd
    ];

    programs.keychain = {
      enable = true;
      extraFlags = [ "--noask" "--quiet" ];
    };

    programs.bacon = { enable = true; };
  };
}
