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
          rust-toolchain = "stable";
        in
        {
          magmawm = final.callPackage ./nix/magmawm.nix {
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
            inputsFrom = [ self.packages.${system}.magmawm ];
            shellHook = ''
              export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${pkgs.libglvnd}/lib"
            '';
          };
        });
    };
}
