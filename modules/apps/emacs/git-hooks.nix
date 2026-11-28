# Installed from the store so checking out a branch can't change the hooks.
{ pkgs, doom, doomConfigLink }:
let
  emptyTree = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

  syncIfChanged = pkgs.writeShellScript "doom-sync-if-changed" ''
    old=$1
    new=$2

    top=$(git rev-parse --show-toplevel) || exit 0
    [ "$(readlink -f "$top/doom")" = "$(readlink -f "${doomConfigLink}")" ] || exit 0

    git rev-parse -q --verify "$old^{tree}" >/dev/null || old=${emptyTree}

    git diff --quiet "$old" "$new" -- doom/init.el doom/packages.el && exit 0

    echo "doom: init.el or packages.el changed, running doom sync..."
    if ${doom} sync; then
      echo "doom: sync done. Restart Emacs to apply (systemctl --user restart emacs)."
    else
      echo "doom: WARNING: doom sync failed; run it manually." >&2
    fi
  '';

  # post-rewrite syncs once after a rebase instead of every step.
  exitDuringRebase = ''
    [ -d "$(git rev-parse --git-path rebase-merge)" ] && exit 0
    [ -d "$(git rev-parse --git-path rebase-apply)" ] && exit 0
  '';

  hooks = {
    # HEAD@{1} also covers amends.
    post-commit = ''
      ${exitDuringRebase}
      exec ${syncIfChanged} 'HEAD@{1}' HEAD
    '';

    post-merge = ''
      exec ${syncIfChanged} ORIG_HEAD HEAD
    '';

    # $3 is 1 for branch checkouts.
    post-checkout = ''
      ${exitDuringRebase}
      [ "$3" = 1 ] || exit 0
      exec ${syncIfChanged} "$1" "$2"
    '';

    # Amends are handled by post-commit.
    post-rewrite = ''
      [ "$1" = rebase ] || exit 0
      exec ${syncIfChanged} ORIG_HEAD HEAD
    '';
  };
in
pkgs.linkFarm "dotfiles-git-hooks" (pkgs.lib.mapAttrs pkgs.writeShellScript hooks)
