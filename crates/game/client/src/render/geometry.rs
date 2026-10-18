//! The derived, GPU-ready geometry library.
//!
//! During client launch every canonical module is baked as both local hierarchies (64-tree and
//! octree) sharing one set of arenas, so the renderer can switch between them at runtime, and the
//! arenas are packed into single storage slabs sized against the device's real limits.
//! Template mini-BVHs are concatenated (with their node references rebased) so every instance of
//! an asset shares its asset's one template.

use crate::{
    lifecycle::load::ClientLaunchLoadingPhase,
    render::{device::GpuCapabilities, gpu::layout::NO_MINI_BVH, scene::rebased},
};
use bevy::{
    ecs::world::CommandQueue, prelude::*, render::extract_resource::ExtractResource,
    tasks::AsyncComputeTaskPool,
};
use common::{
    lifecycle::load::{LoadingAppExt, LoadingTaskComponent, NodeCompleted, StartNode},
    world::VantaAssetLibrary,
};
use std::{sync::Arc, time::Instant};
use voxel::{
    ArenaHandle, Bvh4Node, GeometryArenas, GeometrySource, HierarchyKind, VoxelRoot,
    residency::{ResidencyLimits, plan_single_slab},
};

/// Per-asset GPU addressing.
#[derive(Clone, Debug)]
pub struct AssetGeometry {
    /// Index of the asset's material 0 in the global palette.
    pub palette_base: u32,
    /// Template mini-BVH root in the template node buffer, or `NO_MINI_BVH`.
    pub mini_bvh_root: u32,
    /// Baked roots, indexed by module, then by [`HierarchyKind`].
    pub module_roots: Vec<[VoxelRoot; 2]>,
}

impl AssetGeometry {
    pub fn root(&self, module: u16, kind: HierarchyKind) -> &VoxelRoot {
        &self.module_roots[module as usize][kind as usize]
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct GeometryStats {
    pub voxels: u64,
    /// Nodes per hierarchy, indexed by [`HierarchyKind`].
    pub nodes: [u64; 2],
    pub topology_bytes: u64,
    pub attribute_bytes: u64,
    pub topology_pages: usize,
    pub shared_blocks: u64,
    pub lod_records: usize,
    pub bake_millis: f32,
}

/// Everything the geometry bind group needs, ready to upload.
pub struct GpuGeometryLibrary {
    pub topology: Vec<u32>,
    pub attributes: Vec<u32>,
    pub topology_page_bases: Vec<u32>,
    pub attribute_page_bases: Vec<u32>,
    pub materials: Vec<[f32; 4]>,
    pub template_bvh: Vec<Bvh4Node>,
    /// Indexed like [`VantaAssetLibrary::assets`].
    pub assets: Vec<AssetGeometry>,
    pub stats: GeometryStats,
}

impl GpuGeometryLibrary {
    pub fn build(library: &VantaAssetLibrary, limits: &ResidencyLimits) -> Result<Self, String> {
        let start = Instant::now();
        let mut arenas = GeometryArenas::default();
        let mut materials = Vec::new();
        let mut template_bvh = Vec::new();
        let mut assets = Vec::new();
        let mut stats = GeometryStats::default();

        for asset in library.assets() {
            let mut module_roots = Vec::new();
            for module in &asset.assembly.modules {
                let (tree64, tree64_stats) =
                    arenas.bake_tree64(module).map_err(|e| e.to_string())?;
                let (octree, octree_stats) =
                    arenas.bake_octree(module).map_err(|e| e.to_string())?;
                stats.voxels += tree64_stats.voxels;
                stats.nodes[0] += tree64_stats.nodes;
                stats.nodes[1] += octree_stats.nodes;
                module_roots.push([VoxelRoot::Tree64(tree64), VoxelRoot::Octree(octree)]);
            }

            let palette_base = materials.len() as u32;
            materials.extend(
                asset
                    .assembly
                    .palette
                    .iter()
                    .map(|m| [m.albedo[0], m.albedo[1], m.albedo[2], m.emission]),
            );

            let mini_bvh_root = match &asset.template.mini_bvh {
                Some(bvh) if !bvh.is_empty() => {
                    let base = template_bvh.len() as u32;
                    template_bvh.extend(rebased(&bvh.nodes, base));
                    base
                }
                _ => NO_MINI_BVH,
            };

            assets.push(AssetGeometry {
                palette_base,
                mini_bvh_root,
                module_roots,
            });
        }
        arenas.finish();

        let topology_plan = plan_single_slab(
            "topology",
            arenas.topology.pages().iter().map(Vec::len),
            limits,
        )
        .map_err(|e| e.to_string())?;
        let attribute_plan = plan_single_slab(
            "attribute",
            arenas.attributes.pages().iter().map(Vec::len),
            limits,
        )
        .map_err(|e| e.to_string())?;

        let topology: Vec<u32> = arenas.topology.pages().concat();
        let attributes: Vec<u32> = arenas.attributes.pages().concat();
        stats.topology_bytes = topology.len() as u64 * 4;
        stats.attribute_bytes = attributes.len() as u64 * 4;
        stats.topology_pages = arenas.topology.pages().len();
        stats.shared_blocks =
            arenas.topology.stats().dedup_hits + arenas.attributes.stats().dedup_hits;
        stats.lod_records = arenas.lod.records().len();
        stats.bake_millis = start.elapsed().as_secs_f32() * 1000.0;

        Ok(Self {
            topology,
            attributes,
            topology_page_bases: topology_plan.page_bases,
            attribute_page_bases: attribute_plan.page_bases,
            materials,
            template_bvh,
            assets,
            stats,
        })
    }

    /// The slab base of the page holding `root`'s node.
    pub fn root_page_base(&self, root: &VoxelRoot) -> u32 {
        self.topology_page_bases[root.node().page() as usize]
    }
}

/// The CPU reference can trace the packed slabs exactly as the GPU resolves them.
impl GeometrySource for GpuGeometryLibrary {
    fn topology_word(&self, handle: ArenaHandle) -> u32 {
        self.topology
            [(self.topology_page_bases[handle.page() as usize] + handle.word_offset()) as usize]
    }

    fn attribute_word(&self, handle: ArenaHandle) -> u32 {
        self.attributes
            [(self.attribute_page_bases[handle.page() as usize] + handle.word_offset()) as usize]
    }
}

/// The baked library, shared with the render world.
#[derive(Resource, ExtractResource, Clone)]
pub struct GeometryLibraryHandle(pub Arc<GpuGeometryLibrary>);

/// Client launch loading node that bakes the geometry library.
#[derive(Component)]
pub struct BakeGeometry;

pub struct GeometryPlugin;

impl Plugin for GeometryPlugin {
    fn build(&self, app: &mut App) {
        app.configure_loading_phase::<ClientLaunchLoadingPhase>()
            .add_node(BakeGeometry, start_geometry_bake);
    }
}

fn start_geometry_bake(
    trigger: On<StartNode>,
    mut commands: Commands,
    library: Option<Res<VantaAssetLibrary>>,
    capabilities: Option<Res<GpuCapabilities>>,
) {
    let (Some(library), Some(capabilities)) = (library, capabilities) else {
        error!("Cannot bake geometry without an asset library and a GPU; the world will be empty");
        commands.trigger(NodeCompleted::of::<BakeGeometry>());
        return;
    };
    let library = library.clone();
    let limits = capabilities.residency;
    let node_entity = trigger.0.entity();

    let task = AsyncComputeTaskPool::get().spawn(async move {
        let result = GpuGeometryLibrary::build(&library, &limits);

        let mut queue = CommandQueue::default();
        queue.push(move |world: &mut World| {
            match result {
                Ok(geometry) => {
                    let s = geometry.stats;
                    info!(
                        "Baked geometry: {} voxels, {} 64-tree / {} octree nodes, topology {:.1} MiB in {} page(s), attributes {:.1} MiB, {} shared blocks, in {:.0} ms",
                        s.voxels,
                        s.nodes[0],
                        s.nodes[1],
                        s.topology_bytes as f64 / (1 << 20) as f64,
                        s.topology_pages,
                        s.attribute_bytes as f64 / (1 << 20) as f64,
                        s.shared_blocks,
                        s.bake_millis
                    );
                    world.insert_resource(GeometryLibraryHandle(Arc::new(geometry)));
                }
                Err(e) => error!("Failed to build the GPU geometry library: {}", e),
            }
            world.trigger(NodeCompleted::of::<BakeGeometry>());
        });
        queue
    });

    commands.entity(node_entity).with_children(|parent| {
        parent.spawn(LoadingTaskComponent(task));
    });
}
