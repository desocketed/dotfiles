{ ... }:
{
  imports = [ ./laptop.nix ];

  modules = {
    pass.enable = true;
    syncthing.enable = true;
  };
}
