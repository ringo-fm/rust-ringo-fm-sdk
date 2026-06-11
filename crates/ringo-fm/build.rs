use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let default_swift_pkg = manifest_dir
        .join("..")
        .join("ringo-fm-sys")
        .join("vendor")
        .join("foundation-models-c");
    let swift_pkg = env::var("APPLE_FM_SDK_SWIFT_PKG")
        .map(PathBuf::from)
        .unwrap_or(default_swift_pkg);
    let bin_dir = swift_pkg.join(".build").join("release");

    println!("cargo:rerun-if-env-changed=APPLE_FM_SDK_SWIFT_PKG");
    println!("cargo:rerun-if-changed={}", swift_pkg.join("Package.swift").display());

    let rpath_arg = format!("-Wl,-rpath,{}", bin_dir.display());
    println!("cargo:rustc-link-arg={}", rpath_arg);
    println!("cargo:rustc-link-arg-examples={}", rpath_arg);
}
