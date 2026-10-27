//! Lets a dynamically linked binary on macOS find the toolchain's `libstd` dylib when launched
//! directly (as `brp_launch` does). `cargo run` finds it through `DYLD_FALLBACK_LIBRARY_PATH`, but
//! rustc adds no rpath for it.

use std::{env, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }

    let rustc = env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
    let Some(sysroot) = Command::new(rustc)
        .args(["--print", "sysroot"])
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
    else {
        return;
    };
    let target = env::var("TARGET").unwrap();
    println!(
        "cargo:rustc-link-arg-bins=-Wl,-rpath,{}/lib/rustlib/{target}/lib",
        sysroot.trim()
    );
}
