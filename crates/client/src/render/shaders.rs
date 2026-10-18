// INFO: -------------------------------
//         static shader handles
// -------------------------------------

pub const PRIMARY_VISIBILITY_SHADER: Handle<Shader> =
    uuid_handle!("4b1c7e0a-93d2-4f6b-8a51-2c9e7d3f0b18");
pub const DIRECT_LIGHTING_SHADER: Handle<Shader> =
    uuid_handle!("d82f5a91-6c3e-4e07-b9a4-71f0c2e8d653");

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::asset::uuid_handle;
use bevy::prelude::*;
use wesl::include_wesl;

/// A plugin that registers the WESL shaders (compiled to WGSL by `build.rs`).
pub struct VantablockShaderPlugin;

impl Plugin for VantablockShaderPlugin {
    fn build(&self, app: &mut App) {
        let mut shaders = app.world_mut().resource_mut::<Assets<Shader>>();

        let _ = shaders.insert(
            &PRIMARY_VISIBILITY_SHADER,
            Shader::from_wgsl(
                include_wesl!("primary_visibility"),
                "voxel/primary_visibility.wesl",
            ),
        );
        let _ = shaders.insert(
            &DIRECT_LIGHTING_SHADER,
            Shader::from_wgsl(
                include_wesl!("direct_lighting"),
                "voxel/direct_lighting.wesl",
            ),
        );
    }
}
