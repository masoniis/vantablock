use wesl::Wesl;

fn main() {
    // INFO: --------------------------------------
    //         compile WESL shaders to WGSL
    // --------------------------------------------

    // run condition
    println!("cargo:rerun-if-changed=../../../assets/client/shader");

    let mut compiler = Wesl::new("../../../assets/client/shader");

    // every voxel shader comes in a single-slab variant (the fast path) and a multi-slab variant
    // for geometry spread over several storage bindings
    for (multi_slab, suffix) in [(false, ""), (true, "_multi_slab")] {
        compiler.set_feature("multi_slab", multi_slab);
        for (module, artifact) in [
            ("package::voxel::primary_visibility", "primary_visibility"),
            ("package::voxel::direct_lighting", "direct_lighting"),
            // run on the real adapter by the GPU conformance tests
            ("package::voxel::tests::conformance", "voxel_conformance"),
        ] {
            compiler.build_artifact(&module.parse().unwrap(), &format!("{artifact}{suffix}"));
        }
    }
}
