{mkShell, stdenv, rust-bin, libglvnd, self}: mkShell {
  name = "magmawm";
  # Force linking to libEGL and libwayland-client
  RUSTFLAGS = "-lEGL -lwayland-client";
  LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${libglvnd}/lib";
  inputsFrom = [ self.packages.${stdenv.hostPlatform.system}.magmawm ];
  nativeBuildInputs = [
    rust-bin."stable".latest.default
  ];
};
