{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, rust-overlay, ... }:
    let
      forAllSystems = nixpkgs.lib.genAttrs [
        "aarch64-linux"
        "i686-linux"
        "x86_64-linux"
      ];
      rust-toolchain = "stable";
      pkgsForSystem = system: (import nixpkgs {
        inherit system;
        overlays = [
          (import rust-overlay)
          self.overlays.default
        ];
      });
    in
    {
      overlays.default = final: _prev:
        let
          version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package.version;
        in
        {
          magmawm = final.callPackage ./magmawm.nix {
            inherit (final) lib;
            inherit rust-toolchain version;
            pkgs = final;
          };
        };

      packages = forAllSystems (system: {
        inherit (pkgsForSystem system) magmawm;
        default = self.packages.${system}.magmawm;
      });

      devShells = forAllSystems (system:
        let
          pkgs = pkgsForSystem system;
        in
        {
          default = pkgs.mkShell {
            name = "magmawm";
            NIX_CONFIG = "experimental-features = nix-command flakes";
            # Force linking to libEGL and libwayland-client
            RUSTFLAGS = "-lEGL -lwayland-client";
            LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${pkgs.libglvnd}/lib";
            inputsFrom = [ self.packages.${system}.magmawm ];
            nativeBuildInputs = [
              pkgs.rust-bin."${rust-toolchain}".latest.default
            ];
          };
        });
    };
}
