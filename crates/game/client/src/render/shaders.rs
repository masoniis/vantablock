// INFO: -------------------------------
//         static shader handles
// -------------------------------------

pub const PRIMARY_VISIBILITY_SHADER: Handle<Shader> =
    uuid_handle!("4b1c7e0a-93d2-4f6b-8a51-2c9e7d3f0b18");
pub const DIRECT_LIGHTING_SHADER: Handle<Shader> =
    uuid_handle!("d82f5a91-6c3e-4e07-b9a4-71f0c2e8d653");
/// Variants for geometry spread over several storage slabs.
pub const PRIMARY_VISIBILITY_MULTI_SLAB_SHADER: Handle<Shader> =
    uuid_handle!("7e3a19c4-5b82-4d0f-a6e1-93c84f2b7d05");
pub const DIRECT_LIGHTING_MULTI_SLAB_SHADER: Handle<Shader> =
    uuid_handle!("c1f84e27-0a9d-4b63-8e5f-26d7b9a04c18");
/// Variants with the hardware scene broad phase (single slab, flattened groups only).
pub const PRIMARY_VISIBILITY_RAY_QUERY_SHADER: Handle<Shader> =
    uuid_handle!("2d9b6f13-8e47-4a0c-b5d2-6f1e83a9c704");
pub const DIRECT_LIGHTING_RAY_QUERY_SHADER: Handle<Shader> =
    uuid_handle!("e5a07c3b-19d6-4f82-a3e9-0b74d1c6f258");
/// Primary visibility with footprint LOD filtering, in single-slab, multi-slab, and hardware broad
/// phase forms.
pub const PRIMARY_VISIBILITY_FOOTPRINT_LOD_SHADER: Handle<Shader> =
    uuid_handle!("9a4e2c71-3f06-4b8d-a1c5-58e0d7b39f62");
pub const PRIMARY_VISIBILITY_FOOTPRINT_LOD_MULTI_SLAB_SHADER: Handle<Shader> =
    uuid_handle!("51d7b08e-c2a4-4f39-9e16-0b83f4a6c2d7");
pub const PRIMARY_VISIBILITY_FOOTPRINT_LOD_RAY_QUERY_SHADER: Handle<Shader> =
    uuid_handle!("e83f6a25-7b19-4c0e-8d42-a6c95e1f07b3");
/// Primary visibility with footprint LOD filtering whose filtered hits transmit, in the same three
/// forms.
pub const PRIMARY_VISIBILITY_FOOTPRINT_TRANSMISSION_SHADER: Handle<Shader> =
    uuid_handle!("3c8d1f47-a20e-4b95-9f63-d4e71b0a5c28");
pub const PRIMARY_VISIBILITY_FOOTPRINT_TRANSMISSION_MULTI_SLAB_SHADER: Handle<Shader> =
    uuid_handle!("a6f20b93-4e5c-47d1-8b0a-1c39e7d4f685");
pub const PRIMARY_VISIBILITY_FOOTPRINT_TRANSMISSION_RAY_QUERY_SHADER: Handle<Shader> =
    uuid_handle!("f17e4a08-9d3b-4c62-a5e8-62b0c9d31e74");
/// The temporal resolve (compute) and the pass presenting its image in the view target.
pub const TEMPORAL_RESOLVE_SHADER: Handle<Shader> =
    uuid_handle!("6b2e9d14-0f7a-4c38-b1e5-8d4a3c27f960");
pub const TEMPORAL_PRESENT_SHADER: Handle<Shader> =
    uuid_handle!("c93a5e70-2b18-4d6f-9a04-e17b5d8c3f21");

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::asset::uuid_handle;
use bevy::prelude::*;
use wesl_core::include_wesl;

/// A plugin that registers the WESL shaders (compiled to WGSL by `build.rs`), each in a
/// single-slab, a multi-slab, and a hardware broad phase variant, plus the footprint LOD and
/// footprint transmission forms of primary visibility, and the temporal resolve's two passes.
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
        let _ = shaders.insert(
            &PRIMARY_VISIBILITY_MULTI_SLAB_SHADER,
            Shader::from_wgsl(
                include_wesl!("primary_visibility_multi_slab"),
                "voxel/primary_visibility.wesl (multi slab)",
            ),
        );
        let _ = shaders.insert(
            &DIRECT_LIGHTING_MULTI_SLAB_SHADER,
            Shader::from_wgsl(
                include_wesl!("direct_lighting_multi_slab"),
                "voxel/direct_lighting.wesl (multi slab)",
            ),
        );
        let _ = shaders.insert(
            &PRIMARY_VISIBILITY_RAY_QUERY_SHADER,
            Shader::from_wgsl(
                include_wesl!("primary_visibility_ray_query"),
                "voxel/primary_visibility.wesl (ray query)",
            ),
        );
        let _ = shaders.insert(
            &DIRECT_LIGHTING_RAY_QUERY_SHADER,
            Shader::from_wgsl(
                include_wesl!("direct_lighting_ray_query"),
                "voxel/direct_lighting.wesl (ray query)",
            ),
        );
        let _ = shaders.insert(
            &PRIMARY_VISIBILITY_FOOTPRINT_LOD_SHADER,
            Shader::from_wgsl(
                include_wesl!("primary_visibility_footprint_lod"),
                "voxel/primary_visibility.wesl (footprint lod)",
            ),
        );
        let _ = shaders.insert(
            &PRIMARY_VISIBILITY_FOOTPRINT_LOD_MULTI_SLAB_SHADER,
            Shader::from_wgsl(
                include_wesl!("primary_visibility_footprint_lod_multi_slab"),
                "voxel/primary_visibility.wesl (footprint lod, multi slab)",
            ),
        );
        let _ = shaders.insert(
            &PRIMARY_VISIBILITY_FOOTPRINT_LOD_RAY_QUERY_SHADER,
            Shader::from_wgsl(
                include_wesl!("primary_visibility_footprint_lod_ray_query"),
                "voxel/primary_visibility.wesl (footprint lod, ray query)",
            ),
        );
        let _ = shaders.insert(
            &PRIMARY_VISIBILITY_FOOTPRINT_TRANSMISSION_SHADER,
            Shader::from_wgsl(
                include_wesl!("primary_visibility_footprint_transmission"),
                "voxel/primary_visibility.wesl (footprint transmission)",
            ),
        );
        let _ = shaders.insert(
            &PRIMARY_VISIBILITY_FOOTPRINT_TRANSMISSION_MULTI_SLAB_SHADER,
            Shader::from_wgsl(
                include_wesl!("primary_visibility_footprint_transmission_multi_slab"),
                "voxel/primary_visibility.wesl (footprint transmission, multi slab)",
            ),
        );
        let _ = shaders.insert(
            &PRIMARY_VISIBILITY_FOOTPRINT_TRANSMISSION_RAY_QUERY_SHADER,
            Shader::from_wgsl(
                include_wesl!("primary_visibility_footprint_transmission_ray_query"),
                "voxel/primary_visibility.wesl (footprint transmission, ray query)",
            ),
        );
        let _ = shaders.insert(
            &TEMPORAL_RESOLVE_SHADER,
            Shader::from_wgsl(
                include_wesl!("temporal_resolve"),
                "voxel/temporal_resolve.wesl",
            ),
        );
        let _ = shaders.insert(
            &TEMPORAL_PRESENT_SHADER,
            Shader::from_wgsl(
                include_wesl!("temporal_present"),
                "voxel/temporal_present.wesl",
            ),
        );
    }
}
