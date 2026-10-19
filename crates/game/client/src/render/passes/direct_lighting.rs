use crate::render::{
    VoxelCamera,
    gpu::{
        buffers::{VoxelGpuResources, geometry_layout, scene_layout, scene_ray_query_layout},
        view::{VoxelViewBindGroups, lighting_view_layout},
    },
    scene::ExtractedScene,
    shaders::{
        DIRECT_LIGHTING_MULTI_SLAB_SHADER, DIRECT_LIGHTING_RAY_QUERY_SHADER, DIRECT_LIGHTING_SHADER,
    },
};
use bevy::{
    prelude::*,
    render::{
        diagnostic::RecordDiagnostics,
        render_resource::{
            CachedPipelineState, CachedRenderPipelineId, ColorTargetState, ColorWrites,
            FragmentState, MultisampleState, PipelineCache, PrimitiveState, RenderPassDescriptor,
            RenderPipelineDescriptor, SpecializedRenderPipeline, SpecializedRenderPipelines,
            TextureFormat, VertexState,
        },
        renderer::{RenderContext, ViewQuery},
        view::{Msaa, ViewTarget},
    },
};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct DirectLightingKey {
    pub format: TextureFormat,
    pub samples: u32,
    /// Whether the geometry spans several storage slabs.
    pub multi_slab: bool,
    /// Whether shadow rays use the hardware broad phase (single slab only).
    pub ray_query: bool,
}

#[derive(Resource, Default)]
pub struct DirectLightingPipeline;

impl SpecializedRenderPipeline for DirectLightingPipeline {
    type Key = DirectLightingKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let (shader, scene) = if key.ray_query {
            (DIRECT_LIGHTING_RAY_QUERY_SHADER, scene_ray_query_layout())
        } else if key.multi_slab {
            (DIRECT_LIGHTING_MULTI_SLAB_SHADER, scene_layout())
        } else {
            (DIRECT_LIGHTING_SHADER, scene_layout())
        };
        RenderPipelineDescriptor {
            label: Some("voxel_direct_lighting".into()),
            layout: vec![geometry_layout(), scene, lighting_view_layout()],
            vertex: VertexState {
                shader: shader.clone(),
                shader_defs: vec![],
                entry_point: Some("vs_main".into()),
                buffers: vec![],
            },
            fragment: Some(FragmentState {
                shader,
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
            immediate_size: 0,
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
    resources: Res<VoxelGpuResources>,
    scene: Res<ExtractedScene>,
    mut pipelines: ResMut<SpecializedRenderPipelines<DirectLightingPipeline>>,
    views: Query<(Entity, &ViewTarget, Option<&Msaa>), With<VoxelCamera>>,
) {
    for (entity, target, msaa) in &views {
        let key = DirectLightingKey {
            format: target.main_texture_format(),
            samples: msaa.map_or(1, |m| m.samples()),
            multi_slab: resources.multi_slab(),
            ray_query: resources.traces_with_hardware(&scene.0),
        };
        let id = pipelines.specialize(&pipeline_cache, &pipeline, key);
        commands.entity(entity).insert(DirectLightingPipelineId(id));
    }
}

/// Shades the visibility buffer into the view target. Runs in the `Core3d` schedule of every
/// view; views without voxel targets skip it.
pub fn direct_lighting_pass(
    view: ViewQuery<(&ViewTarget, &VoxelViewBindGroups, &DirectLightingPipelineId)>,
    resources: Res<VoxelGpuResources>,
    pipeline_cache: Res<PipelineCache>,
    mut ctx: RenderContext,
) {
    let (target, bind_groups, pipeline_id) = view.into_inner();
    let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline_id.0) else {
        if let CachedPipelineState::Err(e) = pipeline_cache.get_render_pipeline_state(pipeline_id.0)
        {
            error_once!("Direct lighting pipeline failed: {e}");
        }
        return;
    };

    let diagnostics = ctx.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();
    let mut pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("voxel_direct_lighting"),
        color_attachments: &[Some(target.get_color_attachment())],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    let span = diagnostics.pass_span(&mut pass, "voxel/direct_lighting");
    pass.set_render_pipeline(pipeline);
    pass.set_bind_group(0, resources.geometry_bind_group(), &[]);
    pass.set_bind_group(1, resources.scene_bind_group(), &[]);
    pass.set_bind_group(2, &bind_groups.lighting, &[]);
    pass.draw(0..3, 0..1);
    span.end(&mut pass);
}
