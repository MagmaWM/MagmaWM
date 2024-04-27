{
  	description = "A flake for MagmaWM";

  	inputs = {
		nixpkgs.url = "github:nixos/nixpkgs";
  		rust-overlay = {
  			url = "github:oxalica/rust-overlay";
  			inputs.nixpkgs.follows = "nixpkgs";
  		};
  	};

  outputs = {self, nixpkgs, rust-overlay, ...}: let
    systems = [
      "aarch64-darwin"
      "aarch64-linux"
      "armv6l-linux"
      "armv7l-linux"
      "x86_64-darwin"
      "x86_64-linux"
    ];
    forEachSystem = nixpkgs.lib.genAttrs systems;
    pkgsForEach = nixpkgs.legacyPackages;
  in {
    devShells = forEachSystem (system: {
      default = pkgsForEach.${system}.callPackage ./shell.nix {inherit rust-overlay self;};
    });
    packages = forEachSystem (system: {
      default = pkgsForEach.${system}.callPackage ./default.nix {};
    });
    overlays.default = final: prev: {
      magmawm = final.callPackage ./default.nix {};
    };
  };
}

