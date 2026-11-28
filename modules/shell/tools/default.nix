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
      mosh
      gping
      fzf
      uv
      bfs
      watchexec
      ledger
      hledger
      # nixpkgs topgrade 17.15 sets AWS_LC_SYS_USE_SYSTEM=1 without aws-lc in its inputs (2026-11-08)
      (topgrade.overrideAttrs (o: {
        buildInputs = (o.buildInputs or [ ]) ++ [ aws-lc ];
        env = (o.env or { }) // { AWS_LC_SYS_SYSTEM_DIR = "${runCommand "aws-lc-sysdir" { } "mkdir $out; ln -s ${aws-lc.dev}/include/aws-lc $out/include; ln -s ${aws-lc.out}/lib $out/lib; ln -s ${aws-lc.dev}/share $out/share"}"; };
      }))
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
