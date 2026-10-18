use crate::render::{
    gpu::{
        buffers::{VoxelGpuResources, geometry_layout, scene_layout},
        view::{VisibilityTargets, VoxelViewBindGroups, visibility_view_layout},
    },
    shaders::{PRIMARY_VISIBILITY_MULTI_SLAB_SHADER, PRIMARY_VISIBILITY_SHADER},
};
use bevy::{
    ecs::query::QueryItem,
    prelude::*,
    render::{
        diagnostic::RecordDiagnostics,
        render_graph::{NodeRunError, RenderGraphContext, ViewNode},
        render_resource::{
            CachedComputePipelineId, CachedPipelineState, ComputePassDescriptor,
            ComputePipelineDescriptor, PipelineCache,
        },
        renderer::RenderContext,
    },
};

const WORKGROUP_SIZE: u32 = 8;

/// The single-slab and multi-slab variants; the geometry's residency picks one.
#[derive(Resource)]
pub struct PrimaryVisibilityPipeline {
    pub single_slab: CachedComputePipelineId,
    pub multi_slab: CachedComputePipelineId,
}

impl FromWorld for PrimaryVisibilityPipeline {
    fn from_world(world: &mut World) -> Self {
        let cache = world.resource::<PipelineCache>();
        let queue = |label: &'static str, shader| {
            cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some(label.into()),
                layout: vec![geometry_layout(), scene_layout(), visibility_view_layout()],
                push_constant_ranges: vec![],
                shader,
                shader_defs: vec![],
                entry_point: Some("main".into()),
                zero_initialize_workgroup_memory: false,
            })
        };
        Self {
            single_slab: queue("voxel_primary_visibility", PRIMARY_VISIBILITY_SHADER),
            multi_slab: queue(
                "voxel_primary_visibility_multi_slab",
                PRIMARY_VISIBILITY_MULTI_SLAB_SHADER,
            ),
        }
    }
}

/// Traces one unchanged world ray per pixel into the visibility buffer.
#[derive(Default)]
pub struct PrimaryVisibilityNode;

impl ViewNode for PrimaryVisibilityNode {
    type ViewQuery = (&'static VisibilityTargets, &'static VoxelViewBindGroups);

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (targets, bind_groups): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let resources = world.resource::<VoxelGpuResources>();
        let pipelines = world.resource::<PrimaryVisibilityPipeline>();
        let pipeline_id = if resources.multi_slab() {
            pipelines.multi_slab
        } else {
            pipelines.single_slab
        };
        let pipeline_cache = world.resource::<PipelineCache>();
        let Some(pipeline) = pipeline_cache.get_compute_pipeline(pipeline_id) else {
            if let CachedPipelineState::Err(e) =
                pipeline_cache.get_compute_pipeline_state(pipeline_id)
            {
                error_once!("Primary visibility pipeline failed: {e}");
            }
            return Ok(());
        };

        let diagnostics = render_context.diagnostic_recorder();
        let mut pass =
            render_context
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
        Ok(())
    }
}
