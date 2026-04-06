use wesl::Wesl;

fn main() {
    // INFO: --------------------------------------
    //         compile WESL shaders to WGSL
    // --------------------------------------------

    // run condition
    println!("cargo:rerun-if-changed=src/shaders");

    let compiler = Wesl::new("../../assets/shaders");

    // opaque shaders
    compiler.build_artifact(
        &"package::world::main_passes::opaque::main_vert"
            .parse()
            .unwrap(),
        "opaque_vert",
    );
    compiler.build_artifact(
        &"package::world::main_passes::opaque::main_frag"
            .parse()
            .unwrap(),
        "opaque_frag",
    );

    // skybox shaders
    compiler.build_artifact(
        &"package::world::main_passes::skybox::main_vert"
            .parse()
            .unwrap(),
        "skybox_vert",
    );
    compiler.build_artifact(
        &"package::world::main_passes::skybox::main_frag"
            .parse()
            .unwrap(),
        "skybox_frag",
    );

    // transparent shaders
    compiler.build_artifact(
        &"package::world::main_passes::transparent::main_vert"
            .parse()
            .unwrap(),
        "transparent_vert",
    );
    compiler.build_artifact(
        &"package::world::main_passes::transparent::main_frag"
            .parse()
            .unwrap(),
        "transparent_frag",
    );

    // wireframe shaders
    compiler.build_artifact(
        &"package::world::main_passes::wireframe::main_vert"
            .parse()
            .unwrap(),
        "wireframe_vert",
    );
    compiler.build_artifact(
        &"package::world::main_passes::wireframe::main_frag"
            .parse()
            .unwrap(),
        "wireframe_frag",
    );

    // shadow shaders
    compiler.build_artifact(
        &"package::world::shadow::main_vert".parse().unwrap(),
        "shadow_vert",
    );

    // UI shaders
    compiler.build_artifact(&"package::ui::main_vert".parse().unwrap(), "ui_vert");
    compiler.build_artifact(&"package::ui::main_frag".parse().unwrap(), "ui_frag");
}
