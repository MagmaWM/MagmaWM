{ lib
, pkgs
, version
, ...
}:

pkgs.rustPlatform.buildRustPackage {
  inherit version;
  pname = "magmawm";
  src = lib.cleanSource ./.;

  buildInputs = with pkgs; [
    libdrm
    libglvnd
    libinput
    libseat
    libxkbcommon
    mesa
    pkg-config
    systemdLibs # Contains libudev. DON'T PANIC: it won't install the whole init system
    wayland
    wayland-scanner
    xorg.libX11 # Needed for xwayland to work
    xorg.libXcursor
    xorg.libXi
  ];

  nativeBuildInputs = with pkgs; [
    makeWrapper
    pkg-config
  ];

  cargoLock = {
    lockFile = ./Cargo.lock;
    outputHashes = {
      "smithay-0.3.0" = "sha256-UQwYV3GOkZHEFyAMRMY46JiFXsdsjfSVHAHlA47ZAtI=";
      "smithay-drm-extras-0.1.0" = "sha256-2DrVZ4FiCmAr3DlUfnlb4c1tkcG8ydVHYMG5FUvCTrI=";
      "smithay-egui-0.1.0" = "sha256-FcSoKCwYk3okwQURiQlDUcfk9m/Ne6pSblGAzHDaVHg=";
    };
  };

  postInstall = ''
    wrapProgram $out/bin/magmawm --prefix LD_LIBRARY_PATH : "${pkgs.libglvnd}/lib"
  '';

  meta = {
    description = "A versatile and customizable Window Manager and Wayland Compositor";
    homepage = "https://magmawm.org/";
    license = lib.licenses.mit;
    mainProgram = "magmawm";
  };
}
