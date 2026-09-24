use std::{env, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-env-changed=LG_WEBOS_LEGACY");
    println!("cargo:rerun-if-changed=src/platform/webos_legacy_compat.c");

    if env::var("TARGET").as_deref() != Ok("armv7-unknown-linux-gnueabi")
        || env::var("LG_WEBOS_LEGACY").as_deref() != Ok("1")
    {
        return;
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is set by Cargo"));
    let object = out_dir.join("webos_legacy_compat.o");
    let compiler = env::var("CC_armv7_unknown_linux_gnueabi")
        .or_else(|_| env::var("CC"))
        .unwrap_or_else(|_| "cc".to_string());

    let status = Command::new(&compiler)
        .args(["-fPIC", "-c", "src/platform/webos_legacy_compat.c", "-o"])
        .arg(&object)
        .status()
        .unwrap_or_else(|error| {
            panic!("failed to run {compiler} for webOS legacy compatibility shim: {error}")
        });
    assert!(
        status.success(),
        "{compiler} failed compiling webOS legacy compatibility shim"
    );

    // Keep the shim object late in the final linker command so unresolved libc
    // references from Rust std and native dependencies can resolve to it.
    println!("cargo:rustc-link-arg={}", object.display());
}
