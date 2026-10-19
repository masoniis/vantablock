use wesl::{CompileOptions, Compiler};

const SHADER_ROOT: &str = "../../../assets/client/shader";

/// Compiles the WESL module at `module` with the given feature flags into `$OUT_DIR/{artifact}.wgsl`.
fn build(module: &str, artifact: &str, features: &[(&str, bool)]) {
    let mut options = CompileOptions::default();
    for (name, enabled) in features {
        options.features.set(name, *enabled);
    }
    let result = Compiler::new(options)
        .compile_module(SHADER_ROOT, &module.parse().unwrap())
        .unwrap_or_else(|e| panic!("failed to build WESL shader `{module}`:\n{e}"));
    result.write_artifact(artifact);
}

fn main() {
    // INFO: --------------------------------------
    //         compile WESL shaders to WGSL
    // --------------------------------------------

    // run condition
    println!("cargo:rerun-if-changed={SHADER_ROOT}");

    // every voxel shader comes in a single-slab variant (the fast path) and a multi-slab variant
    // for geometry spread over several storage bindings
    for (multi_slab, suffix) in [(false, ""), (true, "_multi_slab")] {
        for (module, artifact) in [
            ("package::voxel::primary_visibility", "primary_visibility"),
            ("package::voxel::direct_lighting", "direct_lighting"),
            // run on the real adapter by the GPU conformance tests
            ("package::voxel::tests::conformance", "voxel_conformance"),
        ] {
            build(
                module,
                &format!("{artifact}{suffix}"),
                &[("multi_slab", multi_slab), ("stackless", false)],
            );
        }
    }

    // the stackless 64-tree traversal-state experiment, held to the same conformance tests and
    // benchmarked against the default
    build(
        "package::voxel::tests::conformance",
        "voxel_conformance_stackless",
        &[("multi_slab", false), ("stackless", true)],
    );
}
