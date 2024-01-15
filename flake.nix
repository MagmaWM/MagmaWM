{

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = inputs@{ nixpkgs, flake-parts, rust-overlay, ... }:
    flake-parts.lib.mkFlake
      {
        inherit inputs;
      }
      {
        systems = [
          "x86_64-linux"
          "i686-linux"
          "aarch64-linux"
        ];

        perSystem = { system, ... }:
          let
            pkgs = import nixpkgs {
              inherit system;
              overlays = [ (import rust-overlay) ];
            };
            rust-toolchain = "stable";
            rust-package = pkgs.rust-bin."${rust-toolchain}".latest.default;

            native-dependencies = with pkgs; [
              pkg-config
              makeWrapper
            ];

            dependencies = with pkgs; [
              libglvnd
              libseat
              wayland
              wayland-scanner
              pkg-config
              libdrm
              systemdLibs # Contains libudev. DON'T PANIC: it won't install the whole init system
              libxkbcommon
              mesa
              libinput
              xorg.libX11 # Needed for xwayland to work
              xorg.libXcursor
              xorg.libXi
            ];

            # TODO: code this in a split file and import it
            magmawm = pkgs.rustPlatform.buildRustPackage {
              rust = rust-package;
              pname = "magmawm";
              version = "0.0.1";
              src = ./.;
              buildInputs = dependencies;
              nativeBuildInputs = native-dependencies;

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
            };

          in
          {
            devShells.default = pkgs.mkShell {
              buildInputs = dependencies;
              nativeBuildInputs = native-dependencies;

              packages = [
                rust-package
              ];

              shellHook = ''
                export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${pkgs.libglvnd}/lib"
              '';
            };

            packages.default = magmawm;
          };
      };
}

