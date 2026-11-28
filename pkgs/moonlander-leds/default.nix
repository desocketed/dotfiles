{ lib, runCommand, python3, qmkSrc, craneLib, pkg-config, udev, dbus, sqlite, fontconfig, freetype, libxkbcommon, wayland, vulkan-loader }:
let
  # The GUI (gpui) loads these at run time.
  guiLibraries = [ fontconfig freetype libxkbcommon wayland vulkan-loader ];
  # Cargo files, sources and test fixtures only, so other edits keep the build cached.
  src = lib.cleanSourceWith {
    src = ./.;
    filter = path: type:
      craneLib.filterCargoSources path type || lib.hasInfix "/tests/fixtures/" path;
  };
  commonArgs = {
    inherit src;
    strictDeps = true;
    nativeBuildInputs = [ pkg-config ];
    buildInputs = [ udev dbus sqlite ] ++ guiLibraries;
  };
  # src/labels.rs from the keymap and the pinned QMK tree's keyboard.json, so labels cannot drift.
  # The checked-in copy is the fallback for cargo builds outside Nix.
  labels = runCommand "moonlander-labels.rs" { nativeBuildInputs = [ python3 ]; } ''
    board=tree/keyboards/zsa/moonlander
    mkdir -p $board/keymaps/dsktd
    cp ${qmkSrc}/keyboards/zsa/moonlander/keyboard.json $board/
    cp ${../../qmk/keyboards/zsa/moonlander/keymaps/dsktd/keymap.c} $board/keymaps/dsktd/keymap.c
    python3 ${./tools/gen_labels.py} tree dsktd > $out
  '';
  cargoArtifacts = craneLib.buildDepsOnly commonArgs;
in craneLib.buildPackage (commonArgs // {
  inherit cargoArtifacts;
  preBuild = "cp --no-preserve=mode ${labels} src/labels.rs";
  postFixup = ''
    patchelf --add-rpath ${lib.makeLibraryPath guiLibraries} $out/bin/moonlander-leds-gui
  '';
  meta = {
    description = "Drives the Moonlander's LEDs from Niri and MPRIS state over raw HID";
    mainProgram = "moonlander-leds";
    platforms = [ "x86_64-linux" ];
  };
})
