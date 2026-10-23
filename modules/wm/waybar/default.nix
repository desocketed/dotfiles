{ config, lib, ... }:
let
  cfg = config.modules.waybar;
in {
  options.modules.waybar.enable = lib.mkEnableOption "the waybar status bar";

  config = lib.mkIf cfg.enable {
    programs.waybar = {
      systemd.enable = true;
      enable = true;
      settings.mainBar = {
      output = "DP-2";
      position = "top";
      layer = "top";
      height = 30;
      spacing = 0;
      modules-left = [
        "niri/workspaces"
        "custom/lock"
        "custom/reboot"
        "custom/power"
      ];
      modules-center = [ ];
      modules-right = [
        "tray"
        "network"
        "bluetooth"
        "pulseaudio"
        "backlight"
        "custom/temperature"
        "memory"
        "cpu"
        "clock"
      ];
      mpd = {
        format = "{title} ({elapsedTime:%M:%S}) ⸨{songPosition}|{queueLength}⸩ ";
        format-disconnected = "Disconnected ";
        format-stopped = "{consumeIcon}{randomIcon}{repeatIcon}{singleIcon}Stopped ";
        unknown-tag = "N/A";
        interval = 2;
        tooltip-format = "MPD (connected)";
        tooltip-format-disconnected = "MPD (disconnected)";
        on-click = "mpc toggle";
        on-click-right = "systemctl --user restart mpd ; systemctl --user restart mpc";
      };
      idle_inhibitor = {
        format = "{icon}";
        format-icons = {
          activated = "";
          deactivated = "";
        };
      };
      tray = {
        icon-size = 21;
        spacing = 10;
      };
      clock = {
        timezone = config.modules.dotfiles.timezone;
        tooltip-format = "<big>{:%Y %B}</big>\n<tt><small>{calendar}</small></tt>";
        format-alt = "{:%Y-%m-%d}";
        tooltip = false;
      };
      cpu = {
        format = " {usage}%";
        tooltip = false;
      };
      memory = {
        format = " {}%";
      };
      temperature = {
        hwmon-path = "/sys/class/hwmon/hwmon4/temp1_input";
        critical-threshold = 80;
        format = "{temperatureC}°C {icon}";
        format-icons = [ "" "" "" ];
      };
      pulseaudio = {
        format = "{icon}  {volume}%";
        format-muted = " 󰖁 0% ";
        format-icons = {
          headphone = "";
          hands-free = "";
          headset = "";
          phone = "";
          portable = "";
          car = "";
          default = [
            ""
            ""
            ""
          ];
        };
        on-click = "pavucontrol";
      };
      "sway/workspaces" = {
        disable-scroll = true;
        all-outputs = true;
        format = "{name}: {icon}";
        format-icons = {
          urgent = "";
          focused = "";
          default = "";
        };
      };
      "sway/mode" = {
        format = "<span style=\"italic\">{}</span>";
      };
      "niri/workspaces" = {
        all-outputs = true;
        format = "{icon}";
        format-icons = {
          urgent = "";
          focused = "";
          default = "";
        };
      };
      "custom/pipewire" = {
        format = "{icon}";
        return-type = "json";
        signal = 8;
        interval = "once";
        format-icons = {
          mute = "";
          default = [ "" "" "" "" ];
        };
        on-click = "pavucontrol";
        on-scroll-up = "pw-volume change +25%";
        on-scroll-down = "pw-volume change -25%";
        exec = "pw-volume status";
      };
      "custom/notification" = {
        tooltip = false;
        format = "{icon}";
        format-icons = {
          notification = "<span foreground='red'><sup></sup></span>";
          none = " ";
          dnd-notification = "<span foreground='red'><sup></sup></span>";
          dnd-none = "";
          inhibited-notification = "<span foreground='red'><sup></sup></span>";
          inhibited-none = " ";
          dnd-inhibited-notification = "<span foreground='red'><sup></sup></span>";
          dnd-inhibited-none = "";
        };
        return-type = "json";
        exec-if = "which swaync-client";
        exec = "swaync-client -swb";
        on-click = "swaync-client -t -sw";
        on-click-right = "swaync-client -d -sw";
        escape = true;
      };
      "custom/screenshot" = {
        format = "";
        on-click = "grimshot copy area";
        on-click-right = "grimshot copy window";
        on-click-middle = "grimshot copy display";
        tooltip = false;
      };
      bluetooth = {
        format = " {status}";
        format-connected = " {device_alias}";
        format-connected-battery = " {device_alias}{device_battery_percentage}%";
        tooltip-format = "{controller_alias}\t{controller_address}\n\n{num_connections} connected";
        tooltip-format-connected = "{controller_alias}\t{controller_address}\n\n{num_connections} connected\n\n{device_enumerate}";
        tooltip-format-enumerate-connected = "{device_alias}\t{device_address}";
        tooltip-format-enumerate-connected-battery = "{device_alias}\t{device_address}\t{device_battery_percentage}%";
      };
      network = {
        family = "ipv4";
        format-wifi = "󰤨  {essid}";
        format-ethernet = "  Wired ";
        format-linked = "󱘖 {ifname} {ipaddr}/{cidr} (No IP)";
        format-disconnected = " Disconnected";
        format-alt = "󰤨  {ipaddr}/{cidr}";
        tooltip-format = "<span color='#FF1493'> 󰅧 </span>  {bandwidthUpBytes}  <span color='#00BFFF'> 󰅢 </span>  {bandwidthDownBytes}";
        interval = 1;
      };
      backlight = {
        device = "intel_backlight";
        format = "{icon}{percent}%";
        tooltip = true;
        tooltip-format = "Backlight: {percent}%";
        format-icons = [
          "󰃞 "
          "󰃝 "
          "󰃟 "
          "󰃠 "
        ];
      };
      };
      style = ./style.css;
    };
  };
}
