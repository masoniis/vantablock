use wesl::{CompileOptions, Compiler};

const SHADER_ROOT: &str = "../../../assets/client/shader";

/// Compiles the WESL module at `module` into `$OUT_DIR/{artifact}.wgsl`.
fn build(module: &str, artifact: &str) {
    Compiler::new(CompileOptions::default())
        .compile_module(SHADER_ROOT, &module.parse().unwrap())
        .unwrap_or_else(|e| panic!("failed to build WESL shader `{module}`:\n{e}"))
        .write_artifact(artifact);
}

fn main() {
    // INFO: --------------------------------------
    //         compile WESL shaders to WGSL
    // --------------------------------------------

    // run condition
    println!("cargo:rerun-if-changed={SHADER_ROOT}");

    // opaque shaders
    build(
        "package::world::main_passes::opaque::main_vert",
        "opaque_vert",
    );
    build(
        "package::world::main_passes::opaque::main_frag",
        "opaque_frag",
    );

    // skybox shaders
    build(
        "package::world::main_passes::skybox::main_vert",
        "skybox_vert",
    );
    build(
        "package::world::main_passes::skybox::main_frag",
        "skybox_frag",
    );

    // transparent shaders
    build(
        "package::world::main_passes::transparent::main_vert",
        "transparent_vert",
    );
    build(
        "package::world::main_passes::transparent::main_frag",
        "transparent_frag",
    );

    // wireframe shaders
    build(
        "package::world::main_passes::wireframe::main_vert",
        "wireframe_vert",
    );
    build(
        "package::world::main_passes::wireframe::main_frag",
        "wireframe_frag",
    );

    // shadow shaders
    build("package::world::shadow::main_vert", "shadow_vert");
}
