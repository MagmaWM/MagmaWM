{

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = inputs@{ nixpkgs, flake-parts, rust-overlay, ... }:
    flake-parts.lib.mkFlake {
      inherit inputs;
    } {
      systems = [
        "x86_64-linux"
	"i686-linux"
	"aarch64-linux"
      ];
      perSystem = {config, system, ...}: let
        pkgs = import nixpkgs { 
	  inherit system; 
	  overlays = [ (import rust-overlay) ];
	};
	rust-toolchain = "stable";
	
	dependencies = with pkgs; [
            rust-bin."${rust-toolchain}".latest.default
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

      in {
	devShells.default = pkgs.mkShell {
          buildInputs = dependencies;
	};
      };
    };
}

