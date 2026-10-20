//! Copies the toolchain's `std-*.dll` next to the Windows binaries. Dynamically linked bevy needs
//! it, and `cargo run` finds it through `PATH`, but `brp_launch` runs the binary directly.

use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
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
    let lib_dir = PathBuf::from(sysroot.trim()).join(format!("lib/rustlib/{target}/lib"));

    // OUT_DIR is <target>/<profile>/build/<package>-<hash>/out
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let Some(profile_dir) = out_dir.ancestors().nth(3) else {
        return;
    };

    for entry in fs::read_dir(&lib_dir).into_iter().flatten().flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let destination = profile_dir.join(&*name);
        if name.starts_with("std-") && name.ends_with(".dll") && !destination.exists() {
            let _ = fs::copy(entry.path(), destination);
        }
    }
}
