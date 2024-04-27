{ lib,
# Builder
rustPlatform,
# nativeBuildInputs
pkg-config, makeBinaryWrapper,
# buildInputs
libdrm, libglvnd, libinput, libseat, libX11, libXcursor, libXi, libxkbcommon
, mesa, systemdLibs, wayland, wayland-scanner, }:
let
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.intersection
      (lib.fileset.fromSource (lib.sources.cleanSource ./.))
      (lib.fileset.unions [ ./resources ./src ./Cargo.toml ./Cargo.lock ]);
  };
  inherit ((lib.importTOML "${src}/Cargo.toml").package) version;
in rustPlatform.buildRustPackage {
  pname = "magmawm";
  inherit version src;

  buildInputs = [
    libdrm
    libglvnd
    libinput
    libseat
    libxkbcommon
    mesa
    systemdLibs
    wayland
    wayland-scanner
    libX11
    libXcursor
    libXi
  ];

  nativeBuildInputs = [ makeBinaryWrapper pkg-config ];

  cargoLock = {
    lockFile = "${src}/Cargo.lock";
    outputHashes = {
      "smithay-0.3.0" = "sha256-vSzh+qddlJTlclFEyepzjeVeo3WKS9lUysNHr7C2bW0=";
      "smithay-drm-extras-0.1.0" =
        "sha256-2DrVZ4FiCmAr3DlUfnlb4c1tkcG8ydVHYMG5FUvCTrI=";
      "smithay-egui-0.1.0" =
        "sha256-FcSoKCwYk3okwQURiQlDUcfk9m/Ne6pSblGAzHDaVHg=";
    };
  };

  postInstall = ''
    wrapProgram $out/bin/magmawm \
      --prefix LD_LIBRARY_PATH : "${lib.makeLibraryPath [ libglvnd ]}"
  '';

  meta = {
    homepage = "https://magmawm.org/";
    description =
      "A versatile and customizable Window Manager and Wayland Compositor";
    license = lib.licenses.mit;
    mainProgram = "magmawm";
  };
}
