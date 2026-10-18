use crate::render::{
    VoxelCamera,
    gpu::{
        buffers::{VoxelGpuResources, geometry_layout, scene_layout},
        view::{VoxelViewBindGroups, lighting_view_layout},
    },
    shaders::DIRECT_LIGHTING_SHADER,
};
use bevy::{
    ecs::query::QueryItem,
    prelude::*,
    render::{
        diagnostic::RecordDiagnostics,
        render_graph::{NodeRunError, RenderGraphContext, ViewNode},
        render_resource::{
            CachedPipelineState, CachedRenderPipelineId, ColorTargetState, ColorWrites,
            FragmentState, MultisampleState, PipelineCache, PrimitiveState, RenderPassDescriptor,
            RenderPipelineDescriptor, SpecializedRenderPipeline, SpecializedRenderPipelines,
            TextureFormat, VertexState,
        },
        renderer::RenderContext,
        view::{Msaa, ViewTarget},
    },
};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct DirectLightingKey {
    pub format: TextureFormat,
    pub samples: u32,
}

#[derive(Resource, Default)]
pub struct DirectLightingPipeline;

impl SpecializedRenderPipeline for DirectLightingPipeline {
    type Key = DirectLightingKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("voxel_direct_lighting".into()),
            layout: vec![geometry_layout(), scene_layout(), lighting_view_layout()],
            push_constant_ranges: vec![],
            vertex: VertexState {
                shader: DIRECT_LIGHTING_SHADER,
                shader_defs: vec![],
                entry_point: Some("vs_main".into()),
                buffers: vec![],
            },
            fragment: Some(FragmentState {
                shader: DIRECT_LIGHTING_SHADER,
                shader_defs: vec![],
                entry_point: Some("fs_main".into()),
                targets: vec![Some(ColorTargetState {
                    format: key.format,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState {
                count: key.samples,
                ..default()
            },
            zero_initialize_workgroup_memory: false,
        }
    }
}

#[derive(Component)]
pub struct DirectLightingPipelineId(pub CachedRenderPipelineId);

pub fn queue_direct_lighting_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    pipeline: Res<DirectLightingPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<DirectLightingPipeline>>,
    views: Query<(Entity, &ViewTarget, Option<&Msaa>), With<VoxelCamera>>,
) {
    for (entity, target, msaa) in &views {
        let key = DirectLightingKey {
            format: target.main_texture_format(),
            samples: msaa.map_or(1, |m| m.samples()),
        };
        let id = pipelines.specialize(&pipeline_cache, &pipeline, key);
        commands.entity(entity).insert(DirectLightingPipelineId(id));
    }
}

/// Shades the visibility buffer into the view target.
#[derive(Default)]
pub struct DirectLightingNode;

impl ViewNode for DirectLightingNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static VoxelViewBindGroups,
        &'static DirectLightingPipelineId,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (target, bind_groups, pipeline_id): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline_id.0) else {
            if let CachedPipelineState::Err(e) =
                pipeline_cache.get_render_pipeline_state(pipeline_id.0)
            {
                error_once!("Direct lighting pipeline failed: {e}");
            }
            return Ok(());
        };
        let resources = world.resource::<VoxelGpuResources>();

        let diagnostics = render_context.diagnostic_recorder();
        let mut pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("voxel_direct_lighting"),
            color_attachments: &[Some(target.get_color_attachment())],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        let span = diagnostics.pass_span(&mut pass, "voxel/direct_lighting");
        pass.set_render_pipeline(pipeline);
        pass.set_bind_group(0, resources.geometry_bind_group(), &[]);
        pass.set_bind_group(1, resources.scene_bind_group(), &[]);
        pass.set_bind_group(2, &bind_groups.lighting, &[]);
        pass.draw(0..3, 0..1);
        span.end(&mut pass);
        Ok(())
    }
}
