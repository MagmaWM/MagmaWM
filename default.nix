{ lib, pkgs, ...}: let
  pname = "magmawm";
  version = "main";
in
pkgs.rustPlatform.buildRustPackage {
  inherit pname version;
  src = ./.;

  buildInputs = with pkgs; [
    libdrm
    libglvnd
    libinput
    libseat
    libxkbcommon
    mesa
    pkg-config
    systemdLibs
    wayland
    wayland-scanner
    xorg.libX11
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
      "smithay-0.3.0" = "sha256-vSzh+qddlJTlclFEyepzjeVeo3WKS9lUysNHr7C2bW0=";
      "smithay-drm-extras-0.1.0" = "sha256-2DrVZ4FiCmAr3DlUfnlb4c1tkcG8ydVHYMG5FUvCTrI=";
      "smithay-egui-0.1.0" = "sha256-FcSoKCwYk3okwQURiQlDUcfk9m/Ne6pSblGAzHDaVHg=";
    };
  };

  postInstall = ''
    wrapProgram $out/bin/magmawm --prefix LD_LIBRARY_PATH : "${pkgs.libglvnd}/lib"
  '';

  meta = with lib; {
    homepage = "https://magmawm.org/";
    description = "A versatile and customizable Window Manager and Wayland Compositor";
    license = licenses.mit;
    maintainers = with maintainers; [ "HackedOS" "nixos-goddess" ];
    mainProgram = pname;
  };
}


