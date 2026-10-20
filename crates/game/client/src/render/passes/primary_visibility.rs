use crate::render::{
    device::GpuCapabilities,
    gpu::{
        buffers::{VoxelGpuResources, geometry_layout, scene_layout, scene_ray_query_layout},
        view::{VisibilityTargets, VoxelViewBindGroups, visibility_view_layout},
    },
    settings::RenderSettings,
    shaders::{
        PRIMARY_VISIBILITY_FOOTPRINT_LOD_MULTI_SLAB_SHADER,
        PRIMARY_VISIBILITY_FOOTPRINT_LOD_RAY_QUERY_SHADER, PRIMARY_VISIBILITY_FOOTPRINT_LOD_SHADER,
        PRIMARY_VISIBILITY_FOOTPRINT_TRANSMISSION_MULTI_SLAB_SHADER,
        PRIMARY_VISIBILITY_FOOTPRINT_TRANSMISSION_RAY_QUERY_SHADER,
        PRIMARY_VISIBILITY_FOOTPRINT_TRANSMISSION_SHADER, PRIMARY_VISIBILITY_MULTI_SLAB_SHADER,
        PRIMARY_VISIBILITY_RAY_QUERY_SHADER, PRIMARY_VISIBILITY_SHADER,
    },
};
use bevy::{
    prelude::*,
    render::{
        diagnostic::RecordDiagnostics,
        render_resource::{
            CachedComputePipelineId, CachedPipelineState, ComputePassDescriptor,
            ComputePipelineDescriptor, PipelineCache,
        },
        renderer::{RenderContext, ViewQuery},
    },
};

const WORKGROUP_SIZE: u32 = 8;

/// One build of the pass: single-slab and multi-slab variants, which the geometry's residency
/// picks between, and the hardware broad phase variant where the adapter has ray queries.
pub struct PrimaryVisibilityVariants {
    pub single_slab: CachedComputePipelineId,
    pub multi_slab: CachedComputePipelineId,
    pub ray_query: Option<CachedComputePipelineId>,
}

impl PrimaryVisibilityVariants {
    fn select(&self, resources: &VoxelGpuResources) -> CachedComputePipelineId {
        match self.ray_query {
            Some(ray_query) if resources.hardware_broad_phase() => ray_query,
            _ if resources.multi_slab() => self.multi_slab,
            _ => self.single_slab,
        }
    }
}

/// The exact pass, the footprint LOD filtering pass (`RenderSettings::footprint_lod`), and the
/// footprint LOD pass whose filtered hits transmit (`RenderSettings::footprint_transmission`).
#[derive(Resource)]
pub struct PrimaryVisibilityPipeline {
    pub exact: PrimaryVisibilityVariants,
    pub footprint_lod: PrimaryVisibilityVariants,
    pub footprint_transmission: PrimaryVisibilityVariants,
}

impl FromWorld for PrimaryVisibilityPipeline {
    fn from_world(world: &mut World) -> Self {
        let ray_query = world.resource::<GpuCapabilities>().ray_query;
        let cache = world.resource::<PipelineCache>();
        let queue = |label: &'static str, scene, shader| {
            cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some(label.into()),
                layout: vec![geometry_layout(), scene, visibility_view_layout()],
                shader,
                shader_defs: vec![],
                entry_point: Some("main".into()),
                immediate_size: 0,
                zero_initialize_workgroup_memory: false,
            })
        };
        Self {
            exact: PrimaryVisibilityVariants {
                single_slab: queue(
                    "voxel_primary_visibility",
                    scene_layout(),
                    PRIMARY_VISIBILITY_SHADER,
                ),
                multi_slab: queue(
                    "voxel_primary_visibility_multi_slab",
                    scene_layout(),
                    PRIMARY_VISIBILITY_MULTI_SLAB_SHADER,
                ),
                ray_query: ray_query.then(|| {
                    queue(
                        "voxel_primary_visibility_ray_query",
                        scene_ray_query_layout(),
                        PRIMARY_VISIBILITY_RAY_QUERY_SHADER,
                    )
                }),
            },
            footprint_lod: PrimaryVisibilityVariants {
                single_slab: queue(
                    "voxel_primary_visibility_footprint_lod",
                    scene_layout(),
                    PRIMARY_VISIBILITY_FOOTPRINT_LOD_SHADER,
                ),
                multi_slab: queue(
                    "voxel_primary_visibility_footprint_lod_multi_slab",
                    scene_layout(),
                    PRIMARY_VISIBILITY_FOOTPRINT_LOD_MULTI_SLAB_SHADER,
                ),
                ray_query: ray_query.then(|| {
                    queue(
                        "voxel_primary_visibility_footprint_lod_ray_query",
                        scene_ray_query_layout(),
                        PRIMARY_VISIBILITY_FOOTPRINT_LOD_RAY_QUERY_SHADER,
                    )
                }),
            },
            footprint_transmission: PrimaryVisibilityVariants {
                single_slab: queue(
                    "voxel_primary_visibility_footprint_transmission",
                    scene_layout(),
                    PRIMARY_VISIBILITY_FOOTPRINT_TRANSMISSION_SHADER,
                ),
                multi_slab: queue(
                    "voxel_primary_visibility_footprint_transmission_multi_slab",
                    scene_layout(),
                    PRIMARY_VISIBILITY_FOOTPRINT_TRANSMISSION_MULTI_SLAB_SHADER,
                ),
                ray_query: ray_query.then(|| {
                    queue(
                        "voxel_primary_visibility_footprint_transmission_ray_query",
                        scene_ray_query_layout(),
                        PRIMARY_VISIBILITY_FOOTPRINT_TRANSMISSION_RAY_QUERY_SHADER,
                    )
                }),
            },
        }
    }
}

/// Traces one unchanged world ray per pixel into the visibility buffer. Runs in the `Core3d`
/// schedule of every view; views without voxel targets skip it.
pub fn primary_visibility_pass(
    view: ViewQuery<(&VisibilityTargets, &VoxelViewBindGroups)>,
    resources: Res<VoxelGpuResources>,
    pipelines: Res<PrimaryVisibilityPipeline>,
    pipeline_cache: Res<PipelineCache>,
    settings: Option<Res<RenderSettings>>,
    mut ctx: RenderContext,
) {
    let (targets, bind_groups) = view.into_inner();
    let variants = match settings.as_deref() {
        Some(s) if s.footprint_lod && s.footprint_transmission => &pipelines.footprint_transmission,
        Some(s) if s.footprint_lod => &pipelines.footprint_lod,
        _ => &pipelines.exact,
    };
    let pipeline_id = variants.select(&resources);
    let Some(pipeline) = pipeline_cache.get_compute_pipeline(pipeline_id) else {
        if let CachedPipelineState::Err(e) = pipeline_cache.get_compute_pipeline_state(pipeline_id)
        {
            error_once!("Primary visibility pipeline failed: {e}");
        }
        return;
    };

    let diagnostics = ctx.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();
    let mut pass = ctx
        .command_encoder()
        .begin_compute_pass(&ComputePassDescriptor {
            label: Some("voxel_primary_visibility"),
            timestamp_writes: None,
        });
    let span = diagnostics.pass_span(&mut pass, "voxel/primary_visibility");
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, resources.geometry_bind_group(), &[]);
    pass.set_bind_group(1, resources.scene_bind_group(), &[]);
    pass.set_bind_group(2, &bind_groups.visibility, &[]);
    pass.dispatch_workgroups(
        targets.size.x.div_ceil(WORKGROUP_SIZE),
        targets.size.y.div_ceil(WORKGROUP_SIZE),
        1,
    );
    span.end(&mut pass);
}
