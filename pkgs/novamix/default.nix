{ lib, craneLib, rustPlatform, pkg-config, udev, pipewire, makeWrapper
}:
let
  # Cargo files and sources, plus the WirePlumber script the daemon embeds.
  src = lib.cleanSourceWith {
    src = ./.;
    filter = path: type: craneLib.filterCargoSources path type || lib.hasSuffix ".lua" path;
  };
  commonArgs = {
    inherit src;
    pname = "novamix";
    version = "0.1.0";
    strictDeps = true;
    nativeBuildInputs = [ pkg-config rustPlatform.bindgenHook makeWrapper ];
    buildInputs = [ pipewire udev ];
    # Tests need a private PipeWire; tests/route.sh covers them.
    doCheck = false;
  };
  cargoArtifacts = craneLib.buildDepsOnly commonArgs;
in craneLib.buildPackage (commonArgs // {
  inherit cargoArtifacts;
  # The module puts the host's pw-loopback first; this one is only a fallback.
  postInstall = ''
    wrapProgram $out/bin/novamix-daemon --suffix PATH : ${lib.makeBinPath [ pipewire ]}
  '';
  meta = {
    description = "SteelSeries Arctis Nova Pro Wireless ChatMix daemon on PipeWire";
    mainProgram = "novamix-daemon";
    platforms = [ "x86_64-linux" ];
  };
})
