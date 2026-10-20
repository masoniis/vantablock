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
    // naga's ray queries are an enable extension, which WESL can't declare conditionally
    let enables = if features.contains(&("ray_query", true)) {
        "enable wgpu_ray_query;\n"
    } else {
        ""
    };
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let path = std::path::Path::new(&out_dir).join(format!("{artifact}.wgsl"));
    std::fs::write(path, format!("{enables}{result}")).expect("failed to write shader");
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
                &[
                    ("multi_slab", multi_slab),
                    ("stackless", false),
                    ("ray_query", false),
                    ("footprint_lod", false),
                ],
            );
        }
    }

    // the stackless 64-tree traversal-state experiment, held to the same conformance tests and
    // benchmarked against the default
    build(
        "package::voxel::tests::conformance",
        "voxel_conformance_stackless",
        &[
            ("multi_slab", false),
            ("stackless", true),
            ("ray_query", false),
            ("footprint_lod", false),
        ],
    );

    // 64-tree footprint LOD filtering, held to the CPU reference (`tree64::trace_filtered`) by the
    // conformance tests in both residency layouts; no pass uses it yet
    for (multi_slab, suffix) in [(false, ""), (true, "_multi_slab")] {
        build(
            "package::voxel::tests::conformance",
            &format!("voxel_conformance_footprint_lod{suffix}"),
            &[
                ("multi_slab", multi_slab),
                ("stackless", false),
                ("ray_query", false),
                ("footprint_lod", true),
            ],
        );
    }

    // the optional hardware scene broad phase (ray queries over flattened pose groups): the
    // renderer's default on capable adapters, held to the same conformance tests and benchmarked
    // against the software BVH4
    for (module, artifact) in [
        (
            "package::voxel::primary_visibility",
            "primary_visibility_ray_query",
        ),
        (
            "package::voxel::direct_lighting",
            "direct_lighting_ray_query",
        ),
        (
            "package::voxel::tests::conformance",
            "voxel_conformance_ray_query",
        ),
    ] {
        build(
            module,
            artifact,
            &[
                ("multi_slab", false),
                ("stackless", false),
                ("ray_query", true),
                ("footprint_lod", false),
            ],
        );
    }
}
