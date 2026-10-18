use wesl::Wesl;

fn main() {
    // INFO: --------------------------------------
    //         compile WESL shaders to WGSL
    // --------------------------------------------

    // run condition
    println!("cargo:rerun-if-changed=../../../assets/client/shader");

    let compiler = Wesl::new("../../../assets/client/shader");

    for (module, artifact) in [
        ("package::voxel::primary_visibility", "primary_visibility"),
        ("package::voxel::direct_lighting", "direct_lighting"),
        // run on the real adapter by the GPU conformance tests
        ("package::voxel::tests::conformance", "voxel_conformance"),
    ] {
        compiler.build_artifact(&module.parse().unwrap(), artifact);
    }
}
