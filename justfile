upgrade:
    nix flake update --refresh

# Pin the launcher (programs.homun) to its newest published commit; the next switch on each host applies it
bump-homun:
    nix flake update homun --flake path:{{justfile_directory()}}
    just -f {{justfile()}} check

news profile:
    home-manager news --refresh --flake path:{{justfile_directory()}}#{{profile}}

gc:
    home-manager expire-generations "-7 days"
    nix-store --gc
    nix-store --optimise

switch profile:
    home-manager switch --experimental-features 'nix-command flakes' --flake path:{{justfile_directory()}}#{{profile}}

_no-profile:
    @echo "No profile has been selected."

# Evaluate home configurations without building; warnings fail the check.
check *profiles="framework work desktop":
    for p in {{profiles}}; do \
        echo "checking $p"; \
        nix eval --raw --option abort-on-warn true --experimental-features 'nix-command flakes' \
            path:{{justfile_directory()}}#homeConfigurations.$p.activationPackage.drvPath > /dev/null || exit 1; \
    done
