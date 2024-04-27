{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, rust-overlay, ... }:
    let
      inherit (nixpkgs) lib;
      forEachSystem = f:
        (lib.listToAttrs (map (system: {
          name = system;
          value = f {
            inherit system;
            pkgs = import nixpkgs {
              inherit system;
              overlays = [ rust-overlay.overlays.default ];
            };
          };
        }) [
          "aarch64-darwin"
          "aarch64-linux"
          "armv6l-linux"
          "armv7l-linux"
          "x86_64-darwin"
          "x86_64-linux"
        ]));
    in {
      packages = forEachSystem ({ pkgs, system }: {
        magmawm = pkgs.callPackage ./magmawm.nix {
          rustPlatform = let toolchain = pkgs.rust-bin.stable.latest.default;
          in pkgs.makeRustPlatform {
            cargo = toolchain;
            rustc = toolchain;
          };
        };
        default = self.packages.${system}.magmawm;
      });

      devShells = forEachSystem ({ pkgs, system }: {
        default = pkgs.mkShell {
          inputsFrom = [ self.packages.${system}.default ];

          packages = [
            (pkgs.rust-bin.stable.latest.default.override {
              extensions = [ "rustfmt" "rust-analyzer" "clippy" ];
            })
          ];

          env = {
            # Force linking to libEGL and libwayland-client
            RUSTFLAGS = "-lEGL -lwayland-client";
            LD_LIBRARY_PATH = lib.makeLibraryPath [ pkgs.libglvnd ];
          };
        };
      });

      overlays = {
        magmawm = final: _: { magmawm = final.callPackage ./magmawm.nix { }; };

        default = self.overlays.magmawm;
      };

      formatter = forEachSystem ({ pkgs, ... }: pkgs.nixfmt-classic);
    };
}
