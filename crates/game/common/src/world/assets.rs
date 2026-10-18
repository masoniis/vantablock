//! The canonical asset library, loaded during app startup on both client and server.
//!
//! Every `*.ron` recipe in `assets/shared/vanta` is baked into a canonical assembly. Bakes are
//! cached as `.vanta` files keyed by a hash of the recipe and baker version, so unchanged recipes
//! load straight from disk.

use crate::lifecycle::{
    PersistentPathsResource,
    load::{LoadingTaskComponent, NodeCompleted, StartNode},
};
use bevy::{ecs::world::CommandQueue, prelude::*, tasks::AsyncComputeTaskPool};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};
use voxel::{CanonicalAssembly, format, scene::AssetTemplate};

/// The network identity of an asset: a stable hash of its name, so peers agree on IDs no matter
/// what order they loaded assets in.
#[derive(
    Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Reflect,
)]
pub struct VantaAssetId(pub u32);

impl VantaAssetId {
    pub fn from_name(name: &str) -> Self {
        Self(fnv1a(name.as_bytes()) as u32)
    }
}

/// One loaded asset with its derived, shareable search template.
pub struct LoadedAsset {
    pub id: VantaAssetId,
    pub assembly: Arc<CanonicalAssembly>,
    pub template: Arc<AssetTemplate>,
}

/// Every canonical asset known to this peer, in a stable (name-sorted) order.
#[derive(Resource, Default, Clone)]
pub struct VantaAssetLibrary {
    assets: Vec<Arc<LoadedAsset>>,
    by_id: HashMap<VantaAssetId, usize>,
}

impl VantaAssetLibrary {
    pub fn insert(&mut self, assembly: CanonicalAssembly) -> Result<VantaAssetId, String> {
        let id = VantaAssetId::from_name(&assembly.name);
        if let Some(&existing) = self.by_id.get(&id) {
            return Err(format!(
                "asset '{}' has the same id as '{}'",
                assembly.name, self.assets[existing].assembly.name
            ));
        }
        self.by_id.insert(id, self.assets.len());
        self.assets.push(Arc::new(LoadedAsset {
            id,
            template: Arc::new(AssetTemplate::new(&assembly)),
            assembly: Arc::new(assembly),
        }));
        Ok(id)
    }

    pub fn get(&self, id: VantaAssetId) -> Option<&Arc<LoadedAsset>> {
        self.index_of(id).map(|i| &self.assets[i])
    }

    /// The asset's dense index, used to address per-asset tables.
    pub fn index_of(&self, id: VantaAssetId) -> Option<usize> {
        self.by_id.get(&id).copied()
    }

    pub fn id_by_name(&self, name: &str) -> Option<VantaAssetId> {
        let id = VantaAssetId::from_name(name);
        self.by_id.contains_key(&id).then_some(id)
    }

    pub fn assets(&self) -> &[Arc<LoadedAsset>] {
        &self.assets
    }

    pub fn len(&self) -> usize {
        self.assets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }
}

/// Loading node that fills the [`VantaAssetLibrary`].
#[derive(Component)]
pub struct LoadVantaAssets;

pub fn load_vanta_assets(
    trigger: On<StartNode>,
    mut commands: Commands,
    paths: Res<PersistentPathsResource>,
) {
    let recipes_dir = paths.assets_dir.join("shared").join("vanta");
    let cache_dir = paths.cache_dir.join("vanta");
    let node_entity = trigger.0.entity();

    let task = AsyncComputeTaskPool::get().spawn(async move {
        let library = load_library(&recipes_dir, &cache_dir);

        let mut queue = CommandQueue::default();
        queue.push(move |world: &mut World| {
            world.insert_resource(library);
            world.trigger(NodeCompleted::of::<LoadVantaAssets>());
        });
        queue
    });

    commands.entity(node_entity).with_children(|parent| {
        parent.spawn(LoadingTaskComponent(task));
    });
}

/// Bakes (or loads cached bakes of) every recipe in `recipes_dir`. Broken recipes are logged and
/// skipped rather than taking the whole library down.
pub fn load_library(recipes_dir: &Path, cache_dir: &Path) -> VantaAssetLibrary {
    let mut recipes: Vec<PathBuf> = match std::fs::read_dir(recipes_dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|e| e == "ron"))
            .collect(),
        Err(e) => {
            error!("Could not read asset recipes in {:?}: {}", recipes_dir, e);
            Vec::new()
        }
    };
    recipes.sort();

    if let Err(e) = std::fs::create_dir_all(cache_dir) {
        warn!("Could not create asset cache {:?}: {}", cache_dir, e);
    }

    let mut library = VantaAssetLibrary::default();
    for path in recipes {
        match load_asset(&path, cache_dir) {
            Ok(assembly) => {
                let name = assembly.name.clone();
                if let Err(e) = library.insert(assembly) {
                    error!("Skipping asset from {:?}: {}", path, e);
                } else {
                    info!("Loaded asset '{}'", name);
                }
            }
            Err(e) => error!("Failed to load asset recipe {:?}: {}", path, e),
        }
    }
    library
}

fn load_asset(recipe_path: &Path, cache_dir: &Path) -> Result<CanonicalAssembly, String> {
    let text = std::fs::read_to_string(recipe_path).map_err(|e| e.to_string())?;
    let recipe = baker::parse_recipe(&text).map_err(|e| e.to_string())?;

    let key = fnv1a(format!("{text}\0{}\0{}", baker::BAKER_VERSION, format::VERSION).as_bytes());
    let cached = cache_dir.join(format!("{}-{key:016x}.vanta", recipe.name));

    if let Ok(bytes) = std::fs::read(&cached) {
        match format::read(&bytes) {
            Ok(assembly) => return Ok(assembly),
            Err(e) => warn!("Ignoring unreadable cache {:?}: {}", cached, e),
        }
    }

    let start = std::time::Instant::now();
    let assembly = baker::bake_recipe(&recipe);
    info!("Baked '{}' in {:.2?}", recipe.name, start.elapsed());

    // write then rename, so concurrent loaders (singleplayer client and server) never see a
    // partial file
    match format::write(&assembly) {
        Ok(bytes) => {
            let temp = cached.with_extension(format!("tmp{}", std::process::id()));
            if std::fs::write(&temp, bytes).is_ok() {
                let _ = std::fs::rename(&temp, &cached);
            }
        }
        Err(e) => warn!("Could not encode '{}' for the cache: {}", recipe.name, e),
    }
    Ok(assembly)
}

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xCBF2_9CE4_8422_2325, |hash, &b| {
        (hash ^ b as u64).wrapping_mul(0x0000_0100_0000_01B3)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_stable_and_name_based() {
        assert_eq!(
            VantaAssetId::from_name("oak"),
            VantaAssetId::from_name("oak")
        );
        assert_ne!(
            VantaAssetId::from_name("oak"),
            VantaAssetId::from_name("oak2")
        );
    }

    #[test]
    fn shipped_recipes_all_load() {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let cache = std::env::temp_dir().join(format!("vanta-cache-test-{}", std::process::id()));
        let library = load_library(&workspace.join("assets/shared/vanta"), &cache);
        assert!(library.id_by_name("oak").is_some());
        assert!(library.id_by_name("kinetic_rotor").is_some());

        // the second load must come from the cache and be identical
        let again = load_library(&workspace.join("assets/shared/vanta"), &cache);
        for (a, b) in library.assets().iter().zip(again.assets()) {
            assert_eq!(a.assembly, b.assembly);
        }
        let _ = std::fs::remove_dir_all(cache);
    }
}
