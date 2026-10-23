upgrade:
    nix flake update --refresh

news profile:
    home-manager news --refresh --flake path:{{justfile_directory()}}#{{profile}}

gc:
    home-manager expire-generations "-7 days"
    nix-store --gc
    nix-store --optimise

switch profile:
    home-manager switch --refresh --experimental-features 'nix-command flakes' --flake path:{{justfile_directory()}}#{{profile}}

_no-profile:
    @echo "No profile has been selected."

# Evaluate home configurations without building; warnings fail the check.
check *profiles="framework work desktop":
    for p in {{profiles}}; do \
        echo "checking $p"; \
        nix eval --raw --option abort-on-warn true --experimental-features 'nix-command flakes' \
            path:{{justfile_directory()}}#homeConfigurations.$p.activationPackage.drvPath > /dev/null || exit 1; \
    done
