// INFO: -------------------------------
//         static shader handles
// -------------------------------------

// opaque shader handles
pub const OPAQUE_VERT_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("7c4f1e9a-2b5d-4c3f-8e6a-0d2b1f9c8e7a");
pub const OPAQUE_FRAG_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("3e8a1d2c-5b4f-6e9a-0d7c-2b3a1f4e5d6c");
pub const SKYBOX_VERT_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("9b2a5d4c-8e1f-7a0c-3d6f-9b2e5a1c8d4e");
pub const SKYBOX_FRAG_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("2f5e8d1c-4b7a-0d9e-3f6a-2b5d1c4e7f8a");

// transparent shader handles
pub const TRANSPARENT_VERT_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("5a1e8d7c-4b3f-2a9e-0d6c-5b2a0f1e4d7c");
pub const TRANSPARENT_FRAG_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("9d2c5a1b-0e7f-3a6d-4e1f-8c2b5d4a7e9f");

// wireframe shader handles
pub const WIREFRAME_VERT_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("8a1b2c3d-4e5f-6a7b-8c9d-0e1f2a3b4c5d");
pub const WIREFRAME_FRAG_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("4d5e6f7a-8b9c-0d1e-2f3a-4b5c6d7e8f9a");

// shadow shader handles
pub const SHADOW_VERT_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("1a7d4c8e-3b5a-0f2c-7d6e-1b9a4d8c5e3f");

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::asset::uuid_handle;
use bevy::prelude::*;
use wesl::include_wesl;

/// A plugin that registers all the WESL shaders needed for rendering
pub struct VantablockShaderPlugin;

impl Plugin for VantablockShaderPlugin {
    fn build(&self, app: &mut App) {
        let mut shaders = app.world_mut().resource_mut::<Assets<Shader>>();

        // INFO: -------------------------
        //         opaque & skybox
        // -------------------------------

        let _ = shaders.insert(
            &OPAQUE_VERT_SHADER_HANDLE,
            Shader::from_wgsl(include_wesl!("opaque_vert"), "opaque_vert.wesl"),
        );
        let _ = shaders.insert(
            &OPAQUE_FRAG_SHADER_HANDLE,
            Shader::from_wgsl(include_wesl!("opaque_frag"), "opaque_frag.wesl"),
        );

        let _ = shaders.insert(
            &SKYBOX_VERT_SHADER_HANDLE,
            Shader::from_wgsl(include_wesl!("skybox_vert"), "skybox_vert.wesl"),
        );
        let _ = shaders.insert(
            &SKYBOX_FRAG_SHADER_HANDLE,
            Shader::from_wgsl(include_wesl!("skybox_frag"), "skybox_frag.wesl"),
        );

        // INFO: ---------------------
        //         transparent
        // ---------------------------

        let _ = shaders.insert(
            &TRANSPARENT_VERT_SHADER_HANDLE,
            Shader::from_wgsl(include_wesl!("transparent_vert"), "transparent_vert.wesl"),
        );
        let _ = shaders.insert(
            &TRANSPARENT_FRAG_SHADER_HANDLE,
            Shader::from_wgsl(include_wesl!("transparent_frag"), "transparent_frag.wesl"),
        );

        // INFO: -------------------
        //         wireframe
        // -------------------------

        let _ = shaders.insert(
            &WIREFRAME_VERT_SHADER_HANDLE,
            Shader::from_wgsl(include_wesl!("wireframe_vert"), "wireframe_vert.wesl"),
        );
        let _ = shaders.insert(
            &WIREFRAME_FRAG_SHADER_HANDLE,
            Shader::from_wgsl(include_wesl!("wireframe_frag"), "wireframe_frag.wesl"),
        );

        // INFO: ----------------
        //         shadow
        // ----------------------

        let _ = shaders.insert(
            &SHADOW_VERT_SHADER_HANDLE,
            Shader::from_wgsl(include_wesl!("shadow_vert"), "shadow_vert.wesl"),
        );
    }
}
