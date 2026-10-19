//! The hardware scene broad phase in the render world: one shared BLAS of a unit box, and a TLAS
//! with one instance of it per pose frame, mapped onto the frame's padded group-local bounds with
//! the frame index as custom data (see `trace_scene` in `lib/scene.wesl`).
//!
//! Prepare writes the instances whenever a new snapshot arrives; the TLAS is built in the camera
//! schedule ahead of primary visibility, so its GPU time shows up in the frame's pass timings
//! (`VANTA_GPU_TIMINGS=1`) as `voxel/scene_tlas`.

use crate::render::{
    gpu::{buffers::VoxelGpuResources, layout::AffineRows, view::VisibilityTargets},
    scene::GpuSceneData,
};
use bevy::{
    prelude::*,
    render::{
        diagnostic::RecordDiagnostics,
        render_resource::{
            AccelerationStructureFlags, AccelerationStructureGeometryFlags,
            AccelerationStructureUpdateMode, Blas, BlasBuildEntry, BlasGeometries,
            BlasGeometrySizeDescriptors, BlasTriangleGeometry, BlasTriangleGeometrySizeDescriptor,
            Buffer, BufferInitDescriptor, BufferUsages, CreateBlasDescriptor, CreateTlasDescriptor,
            IndexFormat, Tlas, TlasInstance, VertexFormat,
        },
        renderer::{RenderContext, RenderDevice, RenderQueue, ViewQuery},
    },
};
use std::iter;

/// The TLAS instance mask every frame box uses; the shader's ray mask accepts them all.
const INSTANCE_MASK: u8 = 0xFF;

/// The unit box's BLAS, its inputs (kept alive for any later rebuild), and the scene TLAS.
pub struct HardwareBroadPhase {
    blas: Blas,
    tlas: Tlas,
    capacity: u32,
    /// Instances written by the last upload, so a smaller scene can clear the rest.
    instance_count: usize,
    /// Bumped whenever the instances change; the camera schedule builds each generation once.
    generation: u64,
    _box_buffers: [Buffer; 2],
}

impl HardwareBroadPhase {
    /// Creates and builds the unit-box BLAS, with an empty TLAS ready for the first scene.
    pub fn new(device: &RenderDevice, queue: &RenderQueue) -> Self {
        let vertices: Vec<[f32; 3]> = (0..8u32)
            .map(|i| [(i & 1) as f32, (i >> 1 & 1) as f32, (i >> 2 & 1) as f32])
            .collect();
        // two triangles per face; the query culls neither winding
        let faces = [
            [0, 2, 6, 4],
            [1, 3, 7, 5],
            [0, 1, 5, 4],
            [2, 3, 7, 6],
            [0, 1, 3, 2],
            [4, 5, 7, 6],
        ];
        let indices: Vec<u32> = faces
            .iter()
            .flat_map(|[a, b, c, d]| [*a, *b, *c, *a, *c, *d])
            .collect();
        let input = |label, contents: &[u8]| {
            device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some(label),
                contents,
                usage: BufferUsages::BLAS_INPUT,
            })
        };
        let vertex_buffer = input("voxel_unit_box_vertices", bytemuck::cast_slice(&vertices));
        let index_buffer = input("voxel_unit_box_indices", bytemuck::cast_slice(&indices));

        let size = BlasTriangleGeometrySizeDescriptor {
            vertex_format: VertexFormat::Float32x3,
            vertex_count: vertices.len() as u32,
            index_format: Some(IndexFormat::Uint32),
            index_count: Some(indices.len() as u32),
            // not opaque, so every face crossing is a candidate the shader sees
            flags: AccelerationStructureGeometryFlags::empty(),
        };
        let blas = device.wgpu_device().create_blas(
            &CreateBlasDescriptor {
                label: Some("voxel_unit_box"),
                flags: AccelerationStructureFlags::PREFER_FAST_TRACE,
                update_mode: AccelerationStructureUpdateMode::Build,
            },
            BlasGeometrySizeDescriptors::Triangles {
                descriptors: vec![size.clone()],
            },
        );
        let capacity = 1;
        let tlas = create_tlas(device, capacity);

        // the BLAS never changes; build it (and the empty TLAS, so it is valid to bind) now
        let mut encoder = device.create_command_encoder(&default());
        encoder.build_acceleration_structures(
            iter::once(&BlasBuildEntry {
                blas: &blas,
                geometry: BlasGeometries::TriangleGeometries(vec![BlasTriangleGeometry {
                    size: &size,
                    vertex_buffer: &vertex_buffer,
                    first_vertex: 0,
                    vertex_stride: 12,
                    index_buffer: Some(&index_buffer),
                    first_index: Some(0),
                    transform_buffer: None,
                    transform_buffer_offset: None,
                }]),
            }),
            iter::once(&tlas),
        );
        queue.submit([encoder.finish()]);

        Self {
            blas,
            tlas,
            capacity,
            instance_count: 0,
            generation: 0,
            _box_buffers: [vertex_buffer, index_buffer],
        }
    }

    pub fn tlas(&self) -> &Tlas {
        &self.tlas
    }

    /// Writes one instance per frame box of `scene`, returning `true` if the TLAS had to be
    /// reallocated (and so any bind group using it must be rebuilt). The TLAS is built later, by
    /// [`build_scene_tlas`].
    pub fn write(&mut self, device: &RenderDevice, scene: &GpuSceneData) -> bool {
        let boxes = &scene.frame_boxes;
        let mut reallocated = false;
        if boxes.len() > self.capacity as usize {
            self.capacity = (boxes.len() as u32).next_power_of_two();
            self.tlas = create_tlas(device, self.capacity);
            self.instance_count = 0;
            reallocated = true;
        }
        for (frame, frame_box) in boxes.iter().enumerate() {
            self.tlas[frame] = frame_box.map(|rows| instance(&self.blas, rows, frame as u32));
        }
        for frame in boxes.len()..self.instance_count {
            self.tlas[frame] = None;
        }
        self.instance_count = boxes.len();
        self.generation += 1;
        reallocated
    }
}

fn create_tlas(device: &RenderDevice, capacity: u32) -> Tlas {
    device.wgpu_device().create_tlas(&CreateTlasDescriptor {
        label: Some("voxel_scene_frames"),
        max_instances: capacity,
        flags: AccelerationStructureFlags::PREFER_FAST_TRACE,
        update_mode: AccelerationStructureUpdateMode::Build,
    })
}

fn instance(blas: &Blas, rows: AffineRows, frame: u32) -> TlasInstance {
    TlasInstance::new(blas, bytemuck::cast(rows), frame, INSTANCE_MASK)
}

/// Builds the scene TLAS from the instances prepare wrote, once per upload, ahead of the passes
/// that trace it. Runs in the `Core3d` schedule of every voxel view.
pub fn build_scene_tlas(
    _view: ViewQuery<&VisibilityTargets>,
    resources: Res<VoxelGpuResources>,
    mut built: Local<u64>,
    mut ctx: RenderContext,
) {
    let Some(broad_phase) = resources.broad_phase() else {
        return;
    };
    if *built == broad_phase.generation {
        return;
    }
    *built = broad_phase.generation;

    let diagnostics = ctx.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();
    let encoder = ctx.command_encoder();
    let span = diagnostics.time_span(encoder, "voxel/scene_tlas");
    encoder.build_acceleration_structures(iter::empty(), iter::once(&broad_phase.tlas));
    span.end(encoder);
}
