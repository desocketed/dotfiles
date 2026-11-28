{ ... }:
{
  imports = [ ./laptop.nix ];

  programs.homun = {
    enable = true;
    reportRepo = "dsktd/homun-inbox";
    ports = [ { host = 8080; container = 8080; scope = "host"; } ];
  };
}
