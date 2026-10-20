//! The temporal resolve (`RenderSettings::temporal_aa`): after lighting shades the jittered grid,
//! `TemporalResolve` blends it with the reprojected history (`temporal_resolve.wesl`) and
//! `TemporalPresent` shows the result in the view target. Views without the resolve skip both.

use crate::render::{
    VoxelCamera,
    gpu::{
        temporal::{present_layout, resolve_layout},
        view::{TemporalBindGroups, VisibilityTargets},
    },
    passes::direct_lighting::DirectLightingPipelineId,
    settings::RenderSettings,
    shaders::{TEMPORAL_PRESENT_SHADER, TEMPORAL_RESOLVE_SHADER},
};
use bevy::{
    prelude::*,
    render::{
        diagnostic::RecordDiagnostics,
        render_resource::{
            AddressMode, CachedComputePipelineId, CachedPipelineState, CachedRenderPipelineId,
            ColorTargetState, ColorWrites, ComputePassDescriptor, ComputePipelineDescriptor,
            FilterMode, FragmentState, MultisampleState, PipelineCache, PrimitiveState,
            RenderPassDescriptor, RenderPipelineDescriptor, Sampler, SamplerDescriptor,
            SpecializedRenderPipeline, SpecializedRenderPipelines, TextureFormat, VertexState,
        },
        renderer::{RenderContext, RenderDevice, ViewQuery},
        view::{Msaa, ViewTarget},
    },
};

const WORKGROUP_SIZE: u32 = 8;

/// The resolve kernel, the history's bilinear sampler, and the present pass's specializations.
#[derive(Resource)]
pub struct TemporalResolvePipeline {
    pub resolve: CachedComputePipelineId,
    pub sampler: Sampler,
}

impl FromWorld for TemporalResolvePipeline {
    fn from_world(world: &mut World) -> Self {
        let sampler = world
            .resource::<RenderDevice>()
            .create_sampler(&SamplerDescriptor {
                label: Some("voxel_temporal_history_sampler"),
                address_mode_u: AddressMode::ClampToEdge,
                address_mode_v: AddressMode::ClampToEdge,
                mag_filter: FilterMode::Linear,
                min_filter: FilterMode::Linear,
                ..default()
            });
        let resolve =
            world
                .resource::<PipelineCache>()
                .queue_compute_pipeline(ComputePipelineDescriptor {
                    label: Some("voxel_temporal_resolve".into()),
                    layout: vec![resolve_layout()],
                    shader: TEMPORAL_RESOLVE_SHADER,
                    shader_defs: vec![],
                    entry_point: Some("main".into()),
                    immediate_size: 0,
                    zero_initialize_workgroup_memory: false,
                });
        Self { resolve, sampler }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct TemporalPresentKey {
    pub format: TextureFormat,
    pub samples: u32,
}

#[derive(Resource, Default)]
pub struct TemporalPresentPipeline;

impl SpecializedRenderPipeline for TemporalPresentPipeline {
    type Key = TemporalPresentKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("voxel_temporal_present".into()),
            layout: vec![present_layout()],
            vertex: VertexState {
                shader: TEMPORAL_PRESENT_SHADER,
                shader_defs: vec![],
                entry_point: Some("vs_main".into()),
                buffers: vec![],
            },
            fragment: Some(FragmentState {
                shader: TEMPORAL_PRESENT_SHADER,
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
pub struct TemporalPresentPipelineId(pub CachedRenderPipelineId);

pub fn queue_temporal_present_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    pipeline: Res<TemporalPresentPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<TemporalPresentPipeline>>,
    settings: Option<Res<RenderSettings>>,
    views: Query<(Entity, &ViewTarget, Option<&Msaa>), With<VoxelCamera>>,
) {
    // queued before `prepare_temporal_targets` runs, so from the setting it follows
    if !settings.is_some_and(|s| s.temporal_aa) {
        return;
    }
    for (entity, target, msaa) in &views {
        let key = TemporalPresentKey {
            format: target.main_texture_format(),
            samples: msaa.map_or(1, |m| m.samples()),
        };
        let id = pipelines.specialize(&pipeline_cache, &pipeline, key);
        commands
            .entity(entity)
            .insert(TemporalPresentPipelineId(id));
    }
}

/// Whether lighting shaded this view's grid into its temporal targets this frame (its temporal
/// pipeline exists and has compiled), so there is something to resolve.
fn lighting_shaded_grid(lighting: &DirectLightingPipelineId, cache: &PipelineCache) -> bool {
    lighting.temporal && cache.get_render_pipeline(lighting.id).is_some()
}

/// Blends the shaded grid with the reprojected history into this frame's resolved image.
pub fn temporal_resolve_pass(
    view: ViewQuery<(
        &VisibilityTargets,
        &TemporalBindGroups,
        &DirectLightingPipelineId,
    )>,
    pipeline: Res<TemporalResolvePipeline>,
    pipeline_cache: Res<PipelineCache>,
    mut ctx: RenderContext,
) {
    let (targets, bind_groups, lighting) = view.into_inner();
    if !lighting_shaded_grid(lighting, &pipeline_cache) {
        return;
    }
    let Some(compute) = pipeline_cache.get_compute_pipeline(pipeline.resolve) else {
        if let CachedPipelineState::Err(e) =
            pipeline_cache.get_compute_pipeline_state(pipeline.resolve)
        {
            error_once!("Temporal resolve pipeline failed: {e}");
        }
        return;
    };
    let diagnostics = ctx.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();
    let mut pass = ctx
        .command_encoder()
        .begin_compute_pass(&ComputePassDescriptor {
            label: Some("voxel_temporal_resolve"),
            timestamp_writes: None,
        });
    let span = diagnostics.pass_span(&mut pass, "voxel/temporal_resolve");
    pass.set_pipeline(compute);
    pass.set_bind_group(0, &bind_groups.resolve, &[]);
    pass.dispatch_workgroups(
        targets.size.x.div_ceil(WORKGROUP_SIZE),
        targets.size.y.div_ceil(WORKGROUP_SIZE),
        1,
    );
    span.end(&mut pass);
}

/// Shows the resolved image in the view target.
pub fn temporal_present_pass(
    view: ViewQuery<(
        &ViewTarget,
        &TemporalBindGroups,
        &TemporalPresentPipelineId,
        &DirectLightingPipelineId,
    )>,
    pipeline_cache: Res<PipelineCache>,
    mut ctx: RenderContext,
) {
    let (target, bind_groups, pipeline_id, lighting) = view.into_inner();
    if !lighting_shaded_grid(lighting, &pipeline_cache) {
        return;
    }
    let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline_id.0) else {
        if let CachedPipelineState::Err(e) = pipeline_cache.get_render_pipeline_state(pipeline_id.0)
        {
            error_once!("Temporal present pipeline failed: {e}");
        }
        return;
    };
    let mut pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("voxel_temporal_present"),
        color_attachments: &[Some(target.get_color_attachment())],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_render_pipeline(pipeline);
    pass.set_bind_group(0, &bind_groups.present, &[]);
    pass.draw(0..3, 0..1);
}
