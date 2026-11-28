{
  description = "personal dotfiles";

  inputs = {
    crane.url = "github:ipetkov/crane";
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    nur = {
      url = "github:nix-community/NUR";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    home-manager = {
      url = "github:nix-community/home-manager";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    emacs-overlay = {
      url = "github:nix-community/emacs-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    doomemacs = {
      url = "git+https://github.com/doomemacs/core?submodules=1";
      flake = false;
    };
    nixgl = {
      url = "github:nix-community/nixGL";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    qmk_firmware = {
      url = "git+https://github.com/qmk/qmk_firmware?rev=8252b6309752f0be6882c387e15989ba51323d8f&submodules=1";
      flake = false;
    };
    dms = {
      url = "github:AvengeMedia/DankMaterialShell";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    # The launcher version every host runs: `just bump-homun` moves the pin
    homun = {
      url = "github:dsktd/homun";
      flake = false;
    };
  };

  outputs =
    {
      nixpkgs,
      nur,
      home-manager,
      emacs-overlay,
      doomemacs,
      ...
    }@inputs:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};

      mkHost =
        hostFile:
        home-manager.lib.homeManagerConfiguration {
          inherit pkgs;
          extraSpecialArgs = {
            inherit nixpkgs nur emacs-overlay doomemacs;
            inherit (inputs) crane dms nixgl homun qmk_firmware;
          };
          modules = [
            ./hosts/common.nix
            hostFile
          ];
        };
    in
    {
      packages.${system}.moonlander-firmware = pkgs.callPackage ./pkgs/moonlander-firmware.nix {
        qmkSrc = inputs.qmk_firmware;
      };

      homeConfigurations = {
        "framework" = mkHost ./hosts/framework.nix;
        "work" = mkHost ./hosts/work.nix;
        "desktop" = mkHost ./hosts/desktop.nix;
      };
    };
}
