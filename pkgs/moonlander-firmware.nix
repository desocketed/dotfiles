{ lib, stdenv, qmk, gcc-arm-embedded, python3, qmkSrc, userspace ? ../qmk }:
# The dsktd Moonlander keymap, built as a QMK userspace against the pinned upstream QMK tree.
stdenv.mkDerivation {
  pname = "moonlander-firmware";
  version = "0.34.8-3";
  src = qmkSrc;
  nativeBuildInputs = [ qmk gcc-arm-embedded python3 ];
  postPatch = ''
    patchShebangs util
    cp -r --no-preserve=mode ${userspace}/keyboards/. keyboards/
  '';
  buildPhase = ''
    runHook preBuild
    export HOME=$TMPDIR
    make zsa/moonlander:dsktd SKIP_GIT=yes SKIP_VERSION=yes -j$NIX_BUILD_CORES
    runHook postBuild
  '';
  installPhase = ''
    install -Dm644 zsa_moonlander_dsktd.bin $out/zsa_moonlander_dsktd.bin
  '';
}
