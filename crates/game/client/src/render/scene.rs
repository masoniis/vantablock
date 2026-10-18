//! Resolves replicated world objects into a scene snapshot every frame and packs it for the GPU.
//!
//! Poses come from each object's animation parameters and the shared world clock, so nothing
//! per-group is streamed. Canonical voxels never move; only the pose frame table does.

use crate::{
    lifecycle::{ClientLifecycleState, RenderPrepSet},
    render::{
        geometry::{GeometryLibraryHandle, GpuGeometryLibrary},
        gpu::layout::{
            GpuInstance, GpuPoseFrame, GpuSceneUniform, INSTANCE_OWN_BVH, INSTANCE_USES_TEMPLATE,
            MODE_FLATTENED_GROUPS, MODE_INSTANCE_REFIT, MODE_TEMPLATE_MINI_BVH, affine_rows,
        },
        settings::RenderSettings,
    },
};
use bevy::{prelude::*, render::extract_resource::ExtractResource};
use common::{
    time::WorldClockResource,
    world::{ObjectAnimation, ObjectPlacement, VantaAssetLibrary, WorldObject},
};
use std::{sync::Arc, time::Instant};
use voxel::{
    BVH_EMPTY, BVH_LEAF_BIT, Bvh4Node, HierarchyKind, PoseDriver,
    scene::{AssetTemplate, InstanceDesc, SceneAcceleration, SceneSnapshot},
};

/// The packed scene, shared with the render world.
#[derive(Default)]
pub struct GpuSceneData {
    pub uniform: GpuSceneUniform,
    /// The scene BVH, followed by every per-instance refit mini-BVH.
    pub bvh: Vec<Bvh4Node>,
    pub frames: Vec<GpuPoseFrame>,
    pub instances: Vec<GpuInstance>,
    pub slot_frames: Vec<u32>,
}

impl GpuSceneData {
    /// Packs a snapshot for the GPU, addressing each frame's baked `hierarchy` root in `geometry`.
    pub fn pack(
        snapshot: &SceneSnapshot,
        library: &VantaAssetLibrary,
        geometry: &GpuGeometryLibrary,
        hierarchy: HierarchyKind,
    ) -> Self {
        let frames = snapshot
            .frames
            .iter()
            .map(|frame| {
                let asset = &geometry.assets[frame.asset as usize];
                let assembly = &library.assets()[frame.asset as usize].assembly;
                let module = assembly.groups[frame.group as usize].module;
                let root = asset.root(module, hierarchy);
                GpuPoseFrame::new(
                    frame,
                    root,
                    geometry.root_page_base(root),
                    asset.palette_base,
                )
            })
            .collect();
        let mut bvh = snapshot.bvh.nodes.clone();
        let instances = snapshot
            .instances
            .iter()
            .map(|instance| {
                let (mini_bvh_root, flags) = match instance.refit_bvh {
                    Some(index) => {
                        let base = bvh.len() as u32;
                        bvh.extend(rebased(&snapshot.instance_bvhs[index as usize].nodes, base));
                        (base, INSTANCE_OWN_BVH)
                    }
                    None => (
                        geometry.assets[instance.asset as usize].mini_bvh_root,
                        if instance.uses_template {
                            INSTANCE_USES_TEMPLATE
                        } else {
                            0
                        },
                    ),
                };
                GpuInstance {
                    world_to_root: affine_rows(&instance.world_to_root),
                    mini_bvh_root,
                    slot_base: instance.slot_base,
                    group_count: instance.group_count,
                    flags,
                }
            })
            .collect();

        Self {
            uniform: GpuSceneUniform {
                frame_count: snapshot.frames.len() as u32,
                instance_count: snapshot.instances.len() as u32,
                bvh_node_count: snapshot.bvh.nodes.len() as u32,
                mode: match snapshot.mode {
                    SceneAcceleration::TemplateMiniBvh => MODE_TEMPLATE_MINI_BVH,
                    SceneAcceleration::FlattenedGroups => MODE_FLATTENED_GROUPS,
                    SceneAcceleration::InstanceRefit => MODE_INSTANCE_REFIT,
                },
            },
            bvh,
            frames,
            instances,
            slot_frames: snapshot.slot_frames.clone(),
        }
    }
}

/// BVH nodes with their node references offset by `base`, for concatenating trees.
pub fn rebased(nodes: &[Bvh4Node], base: u32) -> impl Iterator<Item = Bvh4Node> + '_ {
    nodes.iter().map(move |node| {
        let mut node = *node;
        for child in &mut node.children {
            if *child != BVH_EMPTY && *child & BVH_LEAF_BIT == 0 {
                *child += base;
            }
        }
        node
    })
}

#[derive(Resource, ExtractResource, Clone, Default)]
pub struct ExtractedScene(pub Arc<GpuSceneData>);

#[derive(Clone, Copy, Debug, Default, Reflect)]
pub struct SceneStats {
    pub objects: usize,
    pub frames: usize,
    pub instances: usize,
    pub bvh_nodes: usize,
    /// Nodes of every per-instance refit mini-BVH (refit mode only).
    pub refit_nodes: usize,
    pub overflowed_frames: usize,
    pub out_of_envelope: usize,
    pub build_millis: f32,
}

/// Client-side scene state carried between frames. Reflected so BRP can read the stats.
#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
pub struct ClientScene {
    /// Search templates in library order, built once the library is available.
    #[reflect(ignore)]
    templates: Vec<AssetTemplate>,
    /// Last frame's snapshot, for motion history.
    #[reflect(ignore)]
    previous: Option<SceneSnapshot>,
    pub stats: SceneStats,
}

pub struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ClientScene>()
            .register_type::<ClientScene>()
            .init_resource::<ExtractedScene>()
            .add_systems(
                PostUpdate,
                build_scene_system
                    .in_set(RenderPrepSet)
                    .run_if(resource_exists::<GeometryLibraryHandle>),
            )
            .add_systems(OnExit(ClientLifecycleState::InGame), clear_scene_system);
    }
}

fn build_scene_system(
    objects: Query<(
        Entity,
        &WorldObject,
        &ObjectPlacement,
        Option<&ObjectAnimation>,
    )>,
    library: Res<VantaAssetLibrary>,
    geometry: Res<GeometryLibraryHandle>,
    clock: Res<WorldClockResource>,
    settings: Res<RenderSettings>,
    mut scene: ResMut<ClientScene>,
    mut extracted: ResMut<ExtractedScene>,
) {
    let start = Instant::now();
    if scene.templates.len() != library.len() {
        scene.templates = library
            .assets()
            .iter()
            .map(|a| (*a.template).clone())
            .collect();
    }

    let time = clock.animation_seconds();
    let instances: Vec<InstanceDesc> = objects
        .iter()
        .filter_map(|(entity, object, placement, animation)| {
            let index = library.index_of(object.asset)?;
            let assembly = &library.assets()[index].assembly;
            let driver: PoseDriver = animation.copied().unwrap_or_default().into();
            Some(InstanceDesc {
                key: entity.to_bits(),
                asset: index as u32,
                root_to_world: placement.root_to_world(assembly.voxel_size),
                pose: driver.evaluate(assembly, time),
            })
        })
        .collect();

    let mode: SceneAcceleration = settings.acceleration.into();
    let snapshot =
        SceneSnapshot::build(&scene.templates, &instances, mode, scene.previous.as_ref());

    let data = GpuSceneData::pack(&snapshot, &library, &geometry.0, settings.hierarchy.into());

    scene.stats = SceneStats {
        objects: objects.iter().len(),
        frames: snapshot.frames.len(),
        instances: snapshot.instances.len(),
        bvh_nodes: snapshot.bvh.nodes.len(),
        refit_nodes: snapshot.instance_bvhs.iter().map(|b| b.nodes.len()).sum(),
        overflowed_frames: snapshot.overflowed_frames,
        // single-root assets never use a template; only count poses that escaped one
        out_of_envelope: snapshot
            .instances
            .iter()
            .filter(|i| !i.uses_template && scene.templates[i.asset as usize].mini_bvh.is_some())
            .count(),
        build_millis: start.elapsed().as_secs_f32() * 1000.0,
    };
    scene.previous = Some(snapshot);
    extracted.0 = Arc::new(data);
}

/// Forgets the previous session's world so it's never drawn behind the menu or the next game.
fn clear_scene_system(mut scene: ResMut<ClientScene>, mut extracted: ResMut<ExtractedScene>) {
    scene.previous = None;
    scene.stats = SceneStats::default();
    extracted.0 = Arc::default();
}
