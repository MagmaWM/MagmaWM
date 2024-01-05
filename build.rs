fn main() {
    // On nixos, the linker cannot find libEGL without this buildscript for some reason
    println!("cargo:rustc-link-lib=EGL");
}
