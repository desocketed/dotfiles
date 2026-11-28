{ config, lib, ... }:
let
  cfg = config.modules.discord;
in {
  options.modules.discord.enable = lib.mkEnableOption "Discord as a Firefox app";

  config = lib.mkIf cfg.enable {

    # /* Hide Tab bar with only one Tab - [110] */
    # #tabbrowser-tabs .tabbrowser-tab:only-of-type,
    # #tabbrowser-tabs .tabbrowser-tab:only-of-type + #tabbrowser-arrowscrollbox-periphery {
    #   display:none !important;
    # }
    # #tabbrowser-tabs, #tabbrowser-arrowscrollbox {min-height:0!important;}

    # #TabsToolbar:not(:hover)
    # #alltabs-button {display:none !important;}

    xdg.desktopEntries.discord = {
      name = "Discord (Firefox/Nix)";
      comment = "Discord as a Firefox app";
      genericName = "Internet Messenger";
      exec = "firefox-nightly -P Discord --name discord --no-remote";
      icon = "discord";
      type = "Application";
      categories = [
        "Network"
        "InstantMessaging"
      ];
    };
    home.file."discord-firefox.desktop" = {
      executable = true;
      target = ".local/share/applications/discord-firefox.desktop";
      text = ''
        [Desktop Entry]
        Name=Discord (Firefox/Nix)
        StartupWMClass=discord
        Comment=All-in-one voice and text chat for gamers that's free, secure, and works on both your desktop and phone.
        GenericName=Internet Messenger
        Exec=/usr/bin/firefox-nightly -P Discord --name discord --no-remote
        Icon=discord
        Type=Application
        Categories=Network;InstantMessaging;
        Path=/usr/bin
      '';
    };
  };
}
