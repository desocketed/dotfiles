{
  description = "personal dotfiles";

  inputs = {
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
          };
          modules = [
            ./hosts/common.nix
            hostFile
          ];
        };
    in
    {
      homeConfigurations = {
        "framework" = mkHost ./hosts/framework.nix;
        "work" = mkHost ./hosts/work.nix;
        "desktop" = mkHost ./hosts/desktop.nix;
      };
    };
}
