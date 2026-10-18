//! Render-world GPU buffers for the geometry library and the per-frame scene, plus the bind group
//! layouts every voxel pass shares (group 0: geometry, group 1: scene).
//!
//! Every buffer is created up front with placeholder contents, so bind groups always exist and
//! the passes can run (drawing just sky) before any world arrives.

use crate::render::{
    geometry::{GeometryLibraryHandle, GpuGeometryLibrary},
    gpu::layout::{GpuGeometryMeta, GpuSceneUniform},
    scene::{ExtractedScene, GpuSceneData},
};
use bevy::{
    prelude::*,
    render::{
        render_resource::{
            BindGroup, BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries, Buffer,
            BufferDescriptor, BufferInitDescriptor, BufferUsages, PipelineCache, ShaderStages,
            binding_types::{storage_buffer_read_only_sized, uniform_buffer_sized},
        },
        renderer::{RenderDevice, RenderQueue},
    },
};
use bytemuck::Pod;
use std::sync::Arc;
use voxel::Bvh4Node;

/// Visibility of the shared groups: traced by the visibility compute pass and by lighting's
/// shadow rays.
const VOXEL_STAGES: ShaderStages = ShaderStages::COMPUTE.union(ShaderStages::FRAGMENT);

pub fn geometry_layout() -> BindGroupLayoutDescriptor {
    BindGroupLayoutDescriptor::new(
        "voxel_geometry_layout",
        &BindGroupLayoutEntries::sequential(
            VOXEL_STAGES,
            (
                storage_buffer_read_only_sized(false, None), // topology
                storage_buffer_read_only_sized(false, None), // attributes
                storage_buffer_read_only_sized(false, None), // materials
                storage_buffer_read_only_sized(false, None), // template mini-BVHs
                uniform_buffer_sized(false, None),           // page tables
            ),
        ),
    )
}

pub fn scene_layout() -> BindGroupLayoutDescriptor {
    BindGroupLayoutDescriptor::new(
        "voxel_scene_layout",
        &BindGroupLayoutEntries::sequential(
            VOXEL_STAGES,
            (
                uniform_buffer_sized(false, None),           // scene uniform
                storage_buffer_read_only_sized(false, None), // scene BVH
                storage_buffer_read_only_sized(false, None), // pose frames
                storage_buffer_read_only_sized(false, None), // instances
                storage_buffer_read_only_sized(false, None), // slot -> frame
            ),
        ),
    )
}

/// A storage buffer that grows (never shrinks) to fit what is written to it.
pub struct GrowableBuffer {
    label: &'static str,
    usage: BufferUsages,
    buffer: Buffer,
    capacity: u64,
}

impl GrowableBuffer {
    pub fn new(device: &RenderDevice, label: &'static str, usage: BufferUsages) -> Self {
        let capacity = 256;
        Self {
            label,
            usage: usage | BufferUsages::COPY_DST,
            buffer: device.create_buffer(&BufferDescriptor {
                label: Some(label),
                size: capacity,
                usage: usage | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            capacity,
        }
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    /// Writes `data`, returning `true` if the buffer had to be reallocated (and so any bind group
    /// using it must be rebuilt).
    pub fn write<T: Pod>(
        &mut self,
        device: &RenderDevice,
        queue: &RenderQueue,
        data: &[T],
    ) -> bool {
        let bytes: &[u8] = bytemuck::cast_slice(data);
        let mut reallocated = false;
        if bytes.len() as u64 > self.capacity {
            self.capacity = (bytes.len() as u64).next_power_of_two();
            self.buffer = device.create_buffer(&BufferDescriptor {
                label: Some(self.label),
                size: self.capacity,
                usage: self.usage,
                mapped_at_creation: false,
            });
            reallocated = true;
        }
        if !bytes.is_empty() {
            queue.write_buffer(&self.buffer, 0, bytes);
        }
        reallocated
    }
}

/// Immutable geometry buffers; rebuilt only when a new library is baked.
struct GeometryBuffers {
    library: Option<Arc<GpuGeometryLibrary>>,
    bind_group: BindGroup,
}

/// Per-frame scene buffers.
struct SceneBuffers {
    uniform: Buffer,
    bvh: GrowableBuffer,
    frames: GrowableBuffer,
    instances: GrowableBuffer,
    slot_frames: GrowableBuffer,
    uploaded: Option<Arc<GpuSceneData>>,
    bind_group: BindGroup,
}

/// The voxel renderer's shared GPU state in the render world.
#[derive(Resource)]
pub struct VoxelGpuResources {
    pub geometry_layout: BindGroupLayoutDescriptor,
    pub scene_layout: BindGroupLayoutDescriptor,
    geometry: GeometryBuffers,
    scene: SceneBuffers,
}

impl VoxelGpuResources {
    pub fn geometry_bind_group(&self) -> &BindGroup {
        &self.geometry.bind_group
    }

    pub fn scene_bind_group(&self) -> &BindGroup {
        &self.scene.bind_group
    }
}

impl FromWorld for VoxelGpuResources {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>().clone();
        let pipeline_cache = world.resource::<PipelineCache>();
        let geometry_layout = geometry_layout();
        let scene_layout = scene_layout();

        let geometry = GeometryBuffers {
            library: None,
            bind_group: create_geometry_bind_group(&device, pipeline_cache, &geometry_layout, None),
        };

        let storage = BufferUsages::STORAGE;
        let uniform = device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("voxel_scene_uniform"),
            contents: bytemuck::bytes_of(&GpuSceneUniform::default()),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let bvh = GrowableBuffer::new(&device, "voxel_scene_bvh", storage);
        let frames = GrowableBuffer::new(&device, "voxel_pose_frames", storage);
        let instances = GrowableBuffer::new(&device, "voxel_instances", storage);
        let slot_frames = GrowableBuffer::new(&device, "voxel_slot_frames", storage);
        let bind_group = create_scene_bind_group(
            &device,
            pipeline_cache,
            &scene_layout,
            &uniform,
            [&bvh, &frames, &instances, &slot_frames],
        );

        Self {
            geometry_layout,
            scene_layout,
            geometry,
            scene: SceneBuffers {
                uniform,
                bvh,
                frames,
                instances,
                slot_frames,
                uploaded: None,
                bind_group,
            },
        }
    }
}

fn storage_init<T: Pod>(device: &RenderDevice, label: &'static str, data: &[T]) -> Buffer {
    // bindings may not be empty
    let bytes: &[u8] = bytemuck::cast_slice(data);
    let padded;
    let contents = if bytes.is_empty() {
        padded = [0u8; 16];
        &padded[..]
    } else {
        bytes
    };
    device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some(label),
        contents,
        usage: BufferUsages::STORAGE,
    })
}

fn create_geometry_bind_group(
    device: &RenderDevice,
    pipeline_cache: &PipelineCache,
    layout: &BindGroupLayoutDescriptor,
    library: Option<&GpuGeometryLibrary>,
) -> BindGroup {
    let empty_bvh = [Bvh4Node::default()];
    let (topology, attributes, materials, bvh, meta) = match library {
        Some(l) => (
            storage_init(device, "voxel_topology", &l.topology),
            storage_init(device, "voxel_attributes", &l.attributes),
            storage_init(device, "voxel_materials", &l.materials),
            storage_init(
                device,
                "voxel_template_bvh",
                if l.template_bvh.is_empty() {
                    &empty_bvh[..]
                } else {
                    &l.template_bvh
                },
            ),
            GpuGeometryMeta::new(&l.topology_page_bases, &l.attribute_page_bases),
        ),
        None => (
            storage_init::<u32>(device, "voxel_topology", &[]),
            storage_init::<u32>(device, "voxel_attributes", &[]),
            storage_init::<[f32; 4]>(device, "voxel_materials", &[]),
            storage_init(device, "voxel_template_bvh", &empty_bvh),
            GpuGeometryMeta::new(&[], &[]),
        ),
    };
    let meta = device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("voxel_geometry_meta"),
        contents: bytemuck::bytes_of(&meta),
        usage: BufferUsages::UNIFORM,
    });
    device.create_bind_group(
        "voxel_geometry_bind_group",
        &pipeline_cache.get_bind_group_layout(layout),
        &BindGroupEntries::sequential((
            topology.as_entire_binding(),
            attributes.as_entire_binding(),
            materials.as_entire_binding(),
            bvh.as_entire_binding(),
            meta.as_entire_binding(),
        )),
    )
}

fn create_scene_bind_group(
    device: &RenderDevice,
    pipeline_cache: &PipelineCache,
    layout: &BindGroupLayoutDescriptor,
    uniform: &Buffer,
    [bvh, frames, instances, slot_frames]: [&GrowableBuffer; 4],
) -> BindGroup {
    device.create_bind_group(
        "voxel_scene_bind_group",
        &pipeline_cache.get_bind_group_layout(layout),
        &BindGroupEntries::sequential((
            uniform.as_entire_binding(),
            bvh.buffer().as_entire_binding(),
            frames.buffer().as_entire_binding(),
            instances.buffer().as_entire_binding(),
            slot_frames.buffer().as_entire_binding(),
        )),
    )
}

/// Uploads a newly baked geometry library.
pub fn prepare_geometry_buffers(
    device: Res<RenderDevice>,
    pipeline_cache: Res<PipelineCache>,
    library: Option<Res<GeometryLibraryHandle>>,
    mut resources: ResMut<VoxelGpuResources>,
) {
    let Some(library) = library else { return };
    if resources
        .geometry
        .library
        .as_ref()
        .is_some_and(|uploaded| Arc::ptr_eq(uploaded, &library.0))
    {
        return;
    }
    let bind_group = create_geometry_bind_group(
        &device,
        &pipeline_cache,
        &resources.geometry_layout,
        Some(&library.0),
    );
    resources.geometry = GeometryBuffers {
        library: Some(library.0.clone()),
        bind_group,
    };
    info!(
        "Uploaded voxel geometry: {:.1} MiB",
        (library.0.topology.len() + library.0.attributes.len()) as f64 * 4.0 / (1 << 20) as f64
    );
}

/// Uploads the scene snapshot whenever a new one was extracted.
pub fn prepare_scene_buffers(
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    pipeline_cache: Res<PipelineCache>,
    scene: Res<ExtractedScene>,
    mut resources: ResMut<VoxelGpuResources>,
) {
    let resources = &mut *resources;
    let buffers = &mut resources.scene;
    if buffers
        .uploaded
        .as_ref()
        .is_some_and(|uploaded| Arc::ptr_eq(uploaded, &scene.0))
    {
        return;
    }
    let data = &scene.0;
    if buffers.uploaded.as_ref().map(|u| u.bvh.len()) != Some(data.bvh.len()) {
        info!(
            "Uploading voxel scene: {} pose frames, {} instances, {} BVH nodes",
            data.frames.len(),
            data.instances.len(),
            data.bvh.len()
        );
    }
    queue.write_buffer(&buffers.uniform, 0, bytemuck::bytes_of(&data.uniform));
    let mut reallocated = buffers.bvh.write(&device, &queue, &data.bvh);
    reallocated |= buffers.frames.write(&device, &queue, &data.frames);
    reallocated |= buffers.instances.write(&device, &queue, &data.instances);
    reallocated |= buffers
        .slot_frames
        .write(&device, &queue, &data.slot_frames);
    if reallocated {
        buffers.bind_group = create_scene_bind_group(
            &device,
            &pipeline_cache,
            &resources.scene_layout,
            &buffers.uniform,
            [
                &buffers.bvh,
                &buffers.frames,
                &buffers.instances,
                &buffers.slot_frames,
            ],
        );
    }
    buffers.uploaded = Some(scene.0.clone());
}
