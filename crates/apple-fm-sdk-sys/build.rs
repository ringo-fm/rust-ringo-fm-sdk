use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let default_swift_pkg = manifest_dir
        .join("..")
        .join("..")
        .join("vendor")
        .join("foundation-models-c");
    let swift_pkg = env::var("APPLE_FM_SDK_SWIFT_PKG")
        .map(PathBuf::from)
        .unwrap_or(default_swift_pkg);

    println!("cargo:rerun-if-env-changed=APPLE_FM_SDK_SWIFT_PKG");
    println!("cargo:rerun-if-changed=wrapper.h");
    let pkg_swift = swift_pkg.join("Package.swift");
    println!("cargo:rerun-if-changed={}", pkg_swift.display());

    let header = swift_pkg
        .join("Sources")
        .join("FoundationModelsCBindings")
        .join("include")
        .join("FoundationModels.h");
    assert!(
        header.exists(),
        "FoundationModels.h not found at {}. Set APPLE_FM_SDK_SWIFT_PKG to the foundation-models-c dir.",
        header.display()
    );
    println!("cargo:rerun-if-changed={}", header.display());

    let status = Command::new("swift")
        .arg("build")
        .arg("-c")
        .arg("release")
        .current_dir(&swift_pkg)
        .status()
        .expect("failed to invoke `swift build` — is Xcode installed?");
    assert!(status.success(), "`swift build` failed in {}", swift_pkg.display());

    let bin_path = Command::new("swift")
        .arg("build")
        .arg("-c")
        .arg("release")
        .arg("--show-bin-path")
        .current_dir(&swift_pkg)
        .output()
        .expect("swift build --show-bin-path failed");
    let bin_dir = String::from_utf8(bin_path.stdout)
        .expect("swift bin path not utf8")
        .trim()
        .to_string();

    println!("cargo:rustc-link-search=native={}", bin_dir);
    println!("cargo:rustc-link-lib=dylib=FoundationModels");

    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg(format!("-I{}", header.parent().unwrap().display()))
        .allowlist_function("FM.*")
        .allowlist_type("FM.*")
        .allowlist_var("FM.*")
        .raw_line("pub type FMTaskRef = *const ::std::os::raw::c_void;")
        .derive_default(true)
        .generate()
        .expect("bindgen failed");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("couldn't write bindings");
}
