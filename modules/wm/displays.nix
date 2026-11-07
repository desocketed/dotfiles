{ lib }:
lib.types.listOf (lib.types.submodule {
  options = {
    name = lib.mkOption {
      type = lib.types.str;
      description = "Output name, as reported by the compositor.";
    };
    main = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = "Whether this is the primary output.";
    };
    bg = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      description = "Background color, e.g. \"#000000\".";
    };
    mode = {
      width = lib.mkOption {
        type = lib.types.int;
        description = "Mode width in pixels.";
      };
      height = lib.mkOption {
        type = lib.types.int;
        description = "Mode height in pixels.";
      };
      rate = lib.mkOption {
        type = lib.types.number;
        description = "Refresh rate in Hz.";
      };
    };
    position = {
      x = lib.mkOption {
        type = lib.types.int;
        default = 0;
        description = "X position in the global output layout.";
      };
      y = lib.mkOption {
        type = lib.types.int;
        default = 0;
        description = "Y position in the global output layout.";
      };
    };
    transform = lib.mkOption {
      type = lib.types.enum [ "normal" "90" "180" "270" "flipped" "flipped-90" "flipped-180" "flipped-270" ];
      default = "normal";
      description = "Output transform, as niri spells it.";
    };
    scale = lib.mkOption {
      type = lib.types.number;
      default = 1.0;
      description = "Output scale factor.";
    };
  };
})
