{
  pkgs,
  config,
  options,
  lib,
  doomemacs,
  ...
}:
let
  cfg = config.modules.emacs;

  emacsPackage =
    (pkgs.emacs-unstable-pgtk.override {
      withTreeSitter = true;
      withGcMarkTrace = false;
      withNativeCompilation = true;
    }).overrideAttrs
      (old: {
        configureFlags = map (
          flag: if flag == "--with-native-compilation" then "--with-native-compilation=aot" else flag
        ) old.configureFlags;
      });

  # See local.nix.example.
  localExtraPackages = if builtins.pathExists ./local.nix then import ./local.nix else (epkgs: [ ]);

  emacsWithPackages = (pkgs.emacsPackagesFor emacsPackage).emacsWithPackages localExtraPackages;

  # DOOMDIR stays unset so Doom uses the mutable ~/.config/doom.
  doomLocalDir = "${config.xdg.dataHome}/doom/local/";

  doomEmacs = pkgs.symlinkJoin {
    name = "doom-emacs-${emacsPackage.version}";
    inherit (emacsPackage) version;
    meta = emacsWithPackages.meta // {
      mainProgram = "emacs";
    };
    paths = [ emacsWithPackages ];
    nativeBuildInputs = [ pkgs.makeBinaryWrapper ];
    postBuild = ''
      cd "$out/bin"
      for bin in emacs emacs-*; do
        [ -e "$bin" ] || continue
        rm "$bin"
        makeWrapper "${emacsWithPackages}/bin/$bin" "$bin" \
          --set-default DOOMLOCALDIR "${doomLocalDir}" \
          --add-flags "--init-directory=${doomemacs}"
      done
      makeWrapper "${doomemacs}/bin/doom" "$out/bin/doom" \
        --set-default DOOMLOCALDIR "${doomLocalDir}" \
        --set EMACS "${emacsWithPackages}/bin/emacs"
    '';
  };
  doom = "${doomEmacs}/bin/doom";
  git = "${pkgs.git}/bin/git";

  doomConfigLink = "${config.xdg.configHome}/doom";
  gitHooksDir = "${config.xdg.configHome}/git/dotfiles-hooks";
  gitHooks = import ./git-hooks.nix { inherit pkgs doom doomConfigLink; };

  treesitGrammars = pkgs.linkFarm "treesit-grammars" [
    {
      name = "libtree-sitter-kdl.so";
      path = "${pkgs.tree-sitter-grammars.tree-sitter-kdl}/parser";
    }
  ];
in
{
  options.modules.emacs = {
    enable = lib.mkEnableOption "Doom Emacs";

    doomDir = lib.mkOption {
      type = lib.types.str;
      default = "${config.modules.dotfiles.dir}/doom";
      description = ''
        Mutable Doom configuration directory, not managed by Nix. It is
        symlinked to ~/.config/doom, and git hooks in its repo run `doom sync`
        when init.el or packages.el change. The default is the doom
        directory of modules.dotfiles.dir.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    assertions = [
      {
        assertion = options.modules.dotfiles.dir.isDefined;
        message = "set modules.dotfiles.dir in hosts/local.nix (absolute path of this checkout)";
      }
    ];

    home.packages = [ doomEmacs ];

    services.emacs = {
      enable = true;
      package = doomEmacs;
    };

    home.file.".local/share/tree-sitter-grammars".source = treesitGrammars;

    xdg.configFile."git/dotfiles-hooks".source = gitHooks;

    home.activation = {
      linkDoomConfig = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
        if [ -e "${doomConfigLink}" ] && [ ! -L "${doomConfigLink}" ]; then
          errorEcho "${doomConfigLink} exists and is not a symlink; move it aside to link ${cfg.doomDir}"
          exit 1
        fi
        if [ ! -d "${cfg.doomDir}" ]; then
          warnEcho "Doom config ${cfg.doomDir} does not exist yet; linking anyway"
        fi
        run mkdir -p $VERBOSE_ARG "${config.xdg.configHome}"
        run ln -sfn $VERBOSE_ARG "${cfg.doomDir}" "${doomConfigLink}"
      '';

      enableDoomGitHooks = lib.hm.dag.entryAfter [ "linkDoomConfig" ] ''
        if ${git} -C "${cfg.doomDir}" rev-parse --git-dir >/dev/null 2>&1; then
          run ${git} -C "${cfg.doomDir}" config core.hooksPath "${gitHooksDir}"
        else
          warnEcho "${cfg.doomDir} is not a git checkout; skipping doom sync hooks"
        fi
      '';

      # Git hooks miss Doom core bumps, so also sync when core or doomDir
      # changes. DOOM_FORCE_SYNC=1 forces it.
      syncDoom = lib.hm.dag.entryBetween [ "reloadSystemd" ] [ "linkDoomConfig" ] ''
        stamp="${config.xdg.dataHome}/doom/synced"
        synced="${doomemacs} $(readlink -f "${cfg.doomDir}")"
        if [ -n "''${DOOM_FORCE_SYNC:-}" ] || [ "$(cat "$stamp" 2>/dev/null)" != "$synced" ]; then
          if [[ -v DRY_RUN ]]; then
            echo "Would run doom sync for $synced"
          else
            # `doom sync` rewrites the envvar file with activation's PATH.
            envFile="${doomLocalDir}etc/init.d/05-doom-env.load.el"
            profileEnvFile="${doomLocalDir}etc/@/init.d/05-doom-env.load.el"
            envBackup=
            if [ -e "$envFile" ]; then
              envBackup=$(mktemp)
              cp "$envFile" "$envBackup"
            fi
            if PATH="${pkgs.git}/bin:$PATH" ${doom} sync; then
              mkdir -p "$(dirname "$stamp")"
              echo "$synced" > "$stamp"
            else
              warnEcho "doom sync failed; it will be retried on the next switch"
            fi
            if [ -n "$envBackup" ]; then
              cp "$envBackup" "$envFile"
              cp "$envBackup" "$profileEnvFile"
              rm "$envBackup"
            fi
          fi
        fi
      '';
    };
  };
}
