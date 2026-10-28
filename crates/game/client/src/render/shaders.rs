//! The world shaders: WESL modules under `assets/client/shader/world`, which Bevy loads as assets
//! (each entry module pulls in its `lib` imports) and composes when a pipeline is queued. Editing a
//! module hot-reloads every pipeline that imports it when the asset watcher is on.
//!
//! [`compose_wgsl`] runs the same composition headlessly, so tests catch a broken shader.

use bevy::{
    asset::uuid::Uuid,
    prelude::*,
    render::RenderApp,
    shader::{ShaderCache, ShaderCacheSource},
};
use std::path::Path;

/// Where the world modules live, relative to the asset root.
pub const SHADER_ROOT: &str = "client/shader/world";

/// An entry module of the world shaders.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WorldShader {
    OpaqueVert,
    OpaqueFrag,
    SkyboxVert,
    SkyboxFrag,
    TransparentVert,
    TransparentFrag,
    WireframeVert,
    WireframeFrag,
    ShadowVert,
}

impl WorldShader {
    pub const ALL: [Self; 9] = [
        Self::OpaqueVert,
        Self::OpaqueFrag,
        Self::SkyboxVert,
        Self::SkyboxFrag,
        Self::TransparentVert,
        Self::TransparentFrag,
        Self::WireframeVert,
        Self::WireframeFrag,
        Self::ShadowVert,
    ];

    /// The module's asset path.
    pub const fn path(self) -> &'static str {
        match self {
            Self::OpaqueVert => "client/shader/world/main_passes/opaque/main_vert.wesl",
            Self::OpaqueFrag => "client/shader/world/main_passes/opaque/main_frag.wesl",
            Self::SkyboxVert => "client/shader/world/main_passes/skybox/main_vert.wesl",
            Self::SkyboxFrag => "client/shader/world/main_passes/skybox/main_frag.wesl",
            Self::TransparentVert => "client/shader/world/main_passes/transparent/main_vert.wesl",
            Self::TransparentFrag => "client/shader/world/main_passes/transparent/main_frag.wesl",
            Self::WireframeVert => "client/shader/world/main_passes/wireframe/main_vert.wesl",
            Self::WireframeFrag => "client/shader/world/main_passes/wireframe/main_frag.wesl",
            Self::ShadowVert => "client/shader/world/shadow/main_vert.wesl",
        }
    }
}

/// Strong handles to every entry module, so they and their imports stay loaded (and hot-reload)
/// for the renderer's lifetime. Lives in the render world, where the pipelines are queued.
#[derive(Resource)]
pub struct WorldShaders([Handle<Shader>; WorldShader::ALL.len()]);

impl WorldShaders {
    pub fn load(assets: &AssetServer) -> Self {
        Self(WorldShader::ALL.map(|shader| assets.load(shader.path())))
    }

    pub fn get(&self, shader: WorldShader) -> Handle<Shader> {
        self.0[shader as usize].clone()
    }
}

/// A plugin that loads the world shaders for the render world's pipelines.
pub struct VantablockShaderPlugin;

impl Plugin for VantablockShaderPlugin {
    fn build(&self, app: &mut App) {
        // loaded at build time, so the handles exist before any pipeline is built in `finish`
        let shaders = WorldShaders::load(app.world().resource::<AssetServer>());
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.insert_resource(shaders);
        }
    }
}

/// Composes `shader` into WGSL the way the pipeline cache does, reading every world module from
/// `assets_dir` (the repository's `assets` in tests).
pub fn compose_wgsl(assets_dir: &Path, shader: WorldShader) -> Result<String, String> {
    let mut cache = ShaderCache::<String, ()>::new((), |_, source, _| match source {
        ShaderCacheSource::Wgsl(wgsl) => Ok(wgsl),
        ShaderCacheSource::SpirV(_) => unreachable!("the world shaders are WESL"),
    });
    let mut modules = vec![assets_dir.join(SHADER_ROOT)];
    let mut entry = None;
    let mut next_id = 1u128;
    while let Some(path) = modules.pop() {
        let entries = std::fs::read_dir(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        for file in entries {
            let file = file.map_err(|e| e.to_string())?.path();
            if file.is_dir() {
                modules.push(file);
                continue;
            }
            if file.extension().is_none_or(|ext| ext != "wesl") {
                continue;
            }
            let source = std::fs::read_to_string(&file).map_err(|e| e.to_string())?;
            let asset_path = file
                .strip_prefix(assets_dir)
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .replace('\\', "/");
            let id = AssetId::Uuid {
                uuid: Uuid::from_u128(next_id),
            };
            next_id += 1;
            if asset_path == shader.path() {
                entry = Some(id);
            }
            cache.set_shader(id, Shader::from_wesl(source, asset_path));
        }
    }
    let entry = entry.ok_or_else(|| format!("{} not found", shader.path()))?;
    cache
        .get(0, entry, &[])
        .map(|wgsl| (*wgsl).clone())
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every entry module composes and passes naga validation, so a broken shader fails the
    /// tests instead of only the running game.
    #[test]
    fn every_world_shader_composes_and_validates() {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../assets");
        for shader in WorldShader::ALL {
            let wgsl = compose_wgsl(&assets, shader)
                .unwrap_or_else(|e| panic!("failed to compose {shader:?}:\n{e}"));
            let module = naga::front::wgsl::parse_str(&wgsl).unwrap_or_else(|e| {
                panic!("failed to parse {shader:?}:\n{}", e.emit_to_string(&wgsl))
            });
            naga::valid::Validator::new(
                naga::valid::ValidationFlags::all(),
                naga::valid::Capabilities::all(),
            )
            .validate(&module)
            .unwrap_or_else(|e| {
                panic!("{shader:?} failed validation:\n{}", e.emit_to_string(&wgsl))
            });
        }
    }
}
