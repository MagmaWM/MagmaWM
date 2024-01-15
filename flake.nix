{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = inputs@{ self, nixpkgs, flake-parts, rust-overlay, ... }:
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
            inherit (pkgs) lib;

            version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package.version;

            pkgs = import nixpkgs {
              inherit system;
              overlays = [ (import rust-overlay) ];
            };

            rust-toolchain = "stable";

            magmawm = pkgs.callPackage ./nix/magmawm.nix {
              inherit pkgs lib rust-toolchain version;
            };

          in
          {
            devShells.default = pkgs.mkShell {
              inputsFrom = [ self.packages.${system}.magmawm ];
              shellHook = ''
                export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${pkgs.libglvnd}/lib"
              '';
            };

            packages.default = self.packages.${system}.magmawm;
            packages.magmawm = magmawm;
          };
      };
}

