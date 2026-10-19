//! Runtime renderer settings, switchable in game for inspection and A/B comparisons.

use crate::input::local_actions::ClientAction;
use bevy::{prelude::*, render::extract_resource::ExtractResource};
use leafwing_input_manager::common_conditions::action_just_pressed;
use voxel::{HierarchyKind, scene::SceneAcceleration};

/// What the lighting pass shows. Indices must match the `DEBUG_*` constants in
/// `assets/client/shader/voxel/direct_lighting.wesl`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
pub enum DebugView {
    #[default]
    Shaded,
    Normals,
    /// One color per resolved pose frame.
    PoseFrames,
    /// One color per exact integer voxel cell.
    VoxelCells,
    /// Voxel DDA steps per pixel.
    TraversalSteps,
    /// Pose groups whose bounds a pixel's ray tested (false-positive bounds show up hot).
    GroupCandidates,
    /// Scene BVH nodes visited per pixel.
    SceneNodes,
    Depth,
    /// Screen-space motion reconstructed from pose-frame and camera history.
    Motion,
}

impl DebugView {
    const ALL: [Self; 9] = [
        Self::Shaded,
        Self::Normals,
        Self::PoseFrames,
        Self::VoxelCells,
        Self::TraversalSteps,
        Self::GroupCandidates,
        Self::SceneNodes,
        Self::Depth,
        Self::Motion,
    ];

    pub fn index(self) -> u32 {
        Self::ALL.iter().position(|&v| v == self).unwrap() as u32
    }

    pub fn next(self) -> Self {
        Self::ALL[(self.index() as usize + 1) % Self::ALL.len()]
    }
}

/// How pose groups are organized for spatial search; all must render identically.
///
/// Flattened groups are the default because they measured fastest (or within a few percent) on
/// both benchmark scenes; template mini-BVHs pay for loose sphere-swept motion envelopes. See the
/// Phase 1 results in `docs/architecture.md`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
pub enum AccelerationMode {
    TemplateMiniBvh,
    #[default]
    FlattenedGroups,
    InstanceRefit,
}

impl From<AccelerationMode> for SceneAcceleration {
    fn from(mode: AccelerationMode) -> Self {
        match mode {
            AccelerationMode::TemplateMiniBvh => SceneAcceleration::TemplateMiniBvh,
            AccelerationMode::FlattenedGroups => SceneAcceleration::FlattenedGroups,
            AccelerationMode::InstanceRefit => SceneAcceleration::InstanceRefit,
        }
    }
}

/// Which local voxel hierarchy pose frames are traced with; both must render identically.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
pub enum LocalHierarchy {
    #[default]
    Tree64,
    Octree,
}

impl From<LocalHierarchy> for HierarchyKind {
    fn from(hierarchy: LocalHierarchy) -> Self {
        match hierarchy {
            LocalHierarchy::Tree64 => HierarchyKind::Tree64,
            LocalHierarchy::Octree => HierarchyKind::Octree,
        }
    }
}

/// Ray grid resolutions, as fractions of the view target, cycled in game.
const RENDER_SCALES: [f32; 4] = [0.25, 0.5, 0.75, 1.0];

#[derive(Resource, ExtractResource, Clone, Copy, Debug, Reflect)]
#[reflect(Resource)]
pub struct RenderSettings {
    pub debug_view: DebugView,
    pub acceleration: AccelerationMode,
    pub hierarchy: LocalHierarchy,
    /// Search flattened pose groups with the hardware broad phase (Vulkan ray queries over a TLAS
    /// of pose-frame boxes) where the adapter has it and the geometry fits one slab. It measured
    /// 25 to 42% faster than the software BVH4, which stays the fallback everywhere else.
    pub hardware_broad_phase: bool,
    /// The traced resolution as a fraction of the view target. Primary visibility costs scale
    /// with its square; lighting upsamples to the target. Defaults to half until traversal
    /// optimization lands (see `just test-gpu gpu_scene_throughput --ignored`).
    pub render_scale: f32,
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self {
            debug_view: DebugView::default(),
            acceleration: AccelerationMode::default(),
            hierarchy: LocalHierarchy::default(),
            hardware_broad_phase: true,
            render_scale: 0.5,
        }
    }
}

pub struct RenderSettingsPlugin;

impl Plugin for RenderSettingsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RenderSettings>()
            .register_type::<RenderSettings>()
            .add_systems(
                Update,
                (
                    cycle_debug_view_system
                        .run_if(action_just_pressed(ClientAction::CycleDebugView)),
                    toggle_acceleration_system
                        .run_if(action_just_pressed(ClientAction::ToggleSceneAcceleration)),
                    cycle_render_scale_system
                        .run_if(action_just_pressed(ClientAction::CycleRenderScale)),
                    toggle_hierarchy_system
                        .run_if(action_just_pressed(ClientAction::ToggleLocalHierarchy)),
                    toggle_hardware_broad_phase_system
                        .run_if(action_just_pressed(ClientAction::ToggleHardwareBroadPhase)),
                ),
            );
    }
}

fn cycle_debug_view_system(mut settings: ResMut<RenderSettings>) {
    settings.debug_view = settings.debug_view.next();
    info!("Debug view: {:?}", settings.debug_view);
}

fn cycle_render_scale_system(mut settings: ResMut<RenderSettings>) {
    let next = RENDER_SCALES
        .iter()
        .position(|&s| s > settings.render_scale + 1e-3)
        .unwrap_or(0);
    settings.render_scale = RENDER_SCALES[next];
    info!("Render scale: {}", settings.render_scale);
}

fn toggle_acceleration_system(mut settings: ResMut<RenderSettings>) {
    settings.acceleration = match settings.acceleration {
        AccelerationMode::TemplateMiniBvh => AccelerationMode::FlattenedGroups,
        AccelerationMode::FlattenedGroups => AccelerationMode::InstanceRefit,
        AccelerationMode::InstanceRefit => AccelerationMode::TemplateMiniBvh,
    };
    info!("Scene acceleration: {:?}", settings.acceleration);
}

fn toggle_hierarchy_system(mut settings: ResMut<RenderSettings>) {
    settings.hierarchy = match settings.hierarchy {
        LocalHierarchy::Tree64 => LocalHierarchy::Octree,
        LocalHierarchy::Octree => LocalHierarchy::Tree64,
    };
    info!("Local hierarchy: {:?}", settings.hierarchy);
}

fn toggle_hardware_broad_phase_system(mut settings: ResMut<RenderSettings>) {
    settings.hardware_broad_phase = !settings.hardware_broad_phase;
    info!(
        "Hardware broad phase: {}",
        if settings.hardware_broad_phase {
            "on where supported"
        } else {
            "off"
        }
    );
}
