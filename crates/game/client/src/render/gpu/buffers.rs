//! Render-world GPU buffers for the geometry library and the per-frame scene, plus the bind group
//! layouts every voxel pass shares (group 0: geometry, group 1: scene). Geometry arenas bind
//! [`MAX_SLABS`] slabs each; unused slab bindings repeat slab 0.
//!
//! Every buffer is created up front with placeholder contents, so bind groups always exist and
//! the passes can run (drawing just sky) before any world arrives.
//!
//! On adapters with ray queries, the scene group also comes in a hardware broad phase layout that
//! adds the scene TLAS ([`HardwareBroadPhase`]); snapshots packed for it are bound with that one.

use crate::render::{
    device::GpuCapabilities,
    geometry::{GeometryLibraryHandle, GpuGeometryLibrary},
    gpu::{
        broad_phase::HardwareBroadPhase,
        layout::{GpuGeometryMeta, GpuSceneUniform},
    },
    scene::{ExtractedScene, GpuSceneData},
};
use bevy::{
    prelude::*,
    render::{
        render_resource::{
            BindGroup, BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries,
            BindingResource, Buffer, BufferDescriptor, BufferInitDescriptor, BufferUsages,
            PipelineCache, ShaderStages,
            binding_types::{
                acceleration_structure, storage_buffer_read_only_sized, uniform_buffer_sized,
            },
        },
        renderer::{RenderDevice, RenderQueue},
    },
};
use bytemuck::Pod;
use std::sync::Arc;
use voxel::{Bvh4Node, residency::MAX_SLABS};

/// Visibility of the shared groups: traced by the visibility compute pass and by lighting's
/// shadow rays.
const VOXEL_STAGES: ShaderStages = ShaderStages::COMPUTE.union(ShaderStages::FRAGMENT);

pub fn geometry_layout() -> BindGroupLayoutDescriptor {
    BindGroupLayoutDescriptor::new(
        "voxel_geometry_layout",
        &BindGroupLayoutEntries::sequential(
            VOXEL_STAGES,
            (
                storage_buffer_read_only_sized(false, None), // topology slab 0
                storage_buffer_read_only_sized(false, None), // attribute slab 0
                storage_buffer_read_only_sized(false, None), // materials
                storage_buffer_read_only_sized(false, None), // template mini-BVHs
                uniform_buffer_sized(false, None),           // page tables
                storage_buffer_read_only_sized(false, None), // topology slab 1
                storage_buffer_read_only_sized(false, None), // topology slab 2
                storage_buffer_read_only_sized(false, None), // topology slab 3
                storage_buffer_read_only_sized(false, None), // attribute slab 1
                storage_buffer_read_only_sized(false, None), // attribute slab 2
                storage_buffer_read_only_sized(false, None), // attribute slab 3
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

/// [`scene_layout`] plus the scene TLAS, for the hardware broad phase shader variants.
pub fn scene_ray_query_layout() -> BindGroupLayoutDescriptor {
    BindGroupLayoutDescriptor::new(
        "voxel_scene_ray_query_layout",
        &BindGroupLayoutEntries::sequential(
            VOXEL_STAGES,
            (
                uniform_buffer_sized(false, None),           // scene uniform
                storage_buffer_read_only_sized(false, None), // scene BVH
                storage_buffer_read_only_sized(false, None), // pose frames
                storage_buffer_read_only_sized(false, None), // instances
                storage_buffer_read_only_sized(false, None), // slot -> frame
                acceleration_structure(),                    // pose-frame boxes
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
    /// The same buffers plus the scene TLAS, on adapters with ray queries.
    hardware_bind_group: Option<BindGroup>,
}

/// The voxel renderer's shared GPU state in the render world.
#[derive(Resource)]
pub struct VoxelGpuResources {
    pub geometry_layout: BindGroupLayoutDescriptor,
    pub scene_layout: BindGroupLayoutDescriptor,
    pub scene_ray_query_layout: BindGroupLayoutDescriptor,
    geometry: GeometryBuffers,
    scene: SceneBuffers,
    /// Present only where the adapter has ray queries.
    broad_phase: Option<HardwareBroadPhase>,
}

impl VoxelGpuResources {
    pub fn geometry_bind_group(&self) -> &BindGroup {
        &self.geometry.bind_group
    }

    /// The scene group for whichever broad phase the uploaded snapshot was packed for.
    pub fn scene_bind_group(&self) -> &BindGroup {
        match &self.scene.hardware_bind_group {
            Some(bind_group) if self.hardware_broad_phase() => bind_group,
            _ => &self.scene.bind_group,
        }
    }

    /// Whether the uploaded geometry spans several slabs, so passes need their multi-slab
    /// shader variants.
    pub fn multi_slab(&self) -> bool {
        self.geometry
            .library
            .as_ref()
            .is_some_and(|l| l.multi_slab())
    }

    /// Whether the uploaded snapshot is traced with the hardware broad phase, so passes need
    /// their ray query shader variants (which are single slab).
    pub fn hardware_broad_phase(&self) -> bool {
        self.scene
            .uploaded
            .as_ref()
            .is_some_and(|scene| self.traces_with_hardware(scene))
    }

    /// Whether `scene` is traced with the hardware broad phase once uploaded. `Queue` runs before
    /// `Prepare`, so pipeline specialization asks this of the extracted snapshot.
    pub fn traces_with_hardware(&self, scene: &GpuSceneData) -> bool {
        scene.hardware_broad_phase && self.broad_phase.is_some() && !self.multi_slab()
    }

    pub fn broad_phase(&self) -> Option<&HardwareBroadPhase> {
        self.broad_phase.as_ref()
    }
}

impl FromWorld for VoxelGpuResources {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>().clone();
        let queue = world.resource::<RenderQueue>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let geometry_layout = geometry_layout();
        let scene_layout = scene_layout();
        let scene_ray_query_layout = scene_ray_query_layout();
        let broad_phase = world
            .resource::<GpuCapabilities>()
            .ray_query
            .then(|| HardwareBroadPhase::new(&device, queue));

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
        let hardware_bind_group = broad_phase.as_ref().map(|broad_phase| {
            create_scene_ray_query_bind_group(
                &device,
                pipeline_cache,
                &scene_ray_query_layout,
                &uniform,
                [&bvh, &frames, &instances, &slot_frames],
                broad_phase,
            )
        });

        Self {
            geometry_layout,
            scene_layout,
            scene_ray_query_layout,
            geometry,
            scene: SceneBuffers {
                uniform,
                bvh,
                frames,
                instances,
                slot_frames,
                uploaded: None,
                bind_group,
                hardware_bind_group,
            },
            broad_phase,
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

/// One buffer per slab, padded with slab 0 so every slab binding is filled.
fn slab_buffers(device: &RenderDevice, label: &'static str, slabs: &[Vec<u32>]) -> Vec<Buffer> {
    let mut buffers: Vec<Buffer> = slabs
        .iter()
        .map(|slab| storage_init(device, label, slab))
        .collect();
    if buffers.is_empty() {
        buffers.push(storage_init::<u32>(device, label, &[]));
    }
    while buffers.len() < MAX_SLABS {
        buffers.push(buffers[0].clone());
    }
    buffers
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
            slab_buffers(device, "voxel_topology", &l.topology),
            slab_buffers(device, "voxel_attributes", &l.attributes),
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
            GpuGeometryMeta::new(&l.topology_page_entries, &l.attribute_page_entries),
        ),
        None => (
            slab_buffers(device, "voxel_topology", &[]),
            slab_buffers(device, "voxel_attributes", &[]),
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
            topology[0].as_entire_binding(),
            attributes[0].as_entire_binding(),
            materials.as_entire_binding(),
            bvh.as_entire_binding(),
            meta.as_entire_binding(),
            topology[1].as_entire_binding(),
            topology[2].as_entire_binding(),
            topology[3].as_entire_binding(),
            attributes[1].as_entire_binding(),
            attributes[2].as_entire_binding(),
            attributes[3].as_entire_binding(),
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

fn create_scene_ray_query_bind_group(
    device: &RenderDevice,
    pipeline_cache: &PipelineCache,
    layout: &BindGroupLayoutDescriptor,
    uniform: &Buffer,
    [bvh, frames, instances, slot_frames]: [&GrowableBuffer; 4],
    broad_phase: &HardwareBroadPhase,
) -> BindGroup {
    device.create_bind_group(
        "voxel_scene_ray_query_bind_group",
        &pipeline_cache.get_bind_group_layout(layout),
        &BindGroupEntries::sequential((
            uniform.as_entire_binding(),
            bvh.buffer().as_entire_binding(),
            frames.buffer().as_entire_binding(),
            instances.buffer().as_entire_binding(),
            slot_frames.buffer().as_entire_binding(),
            BindingResource::AccelerationStructure(broad_phase.tlas()),
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
    let words: usize = library
        .0
        .topology
        .iter()
        .chain(&library.0.attributes)
        .map(Vec::len)
        .sum();
    info!(
        "Uploaded voxel geometry: {:.1} MiB",
        words as f64 * 4.0 / (1 << 20) as f64
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
    let scene_buffers = [
        &buffers.bvh,
        &buffers.frames,
        &buffers.instances,
        &buffers.slot_frames,
    ];
    if reallocated {
        buffers.bind_group = create_scene_bind_group(
            &device,
            &pipeline_cache,
            &resources.scene_layout,
            &buffers.uniform,
            scene_buffers,
        );
    }
    if let Some(broad_phase) = &mut resources.broad_phase {
        // only snapshots packed for the hardware path carry frame boxes
        let tlas_reallocated = data.hardware_broad_phase && broad_phase.write(&device, data);
        if reallocated || tlas_reallocated {
            buffers.hardware_bind_group = Some(create_scene_ray_query_bind_group(
                &device,
                &pipeline_cache,
                &resources.scene_ray_query_layout,
                &buffers.uniform,
                scene_buffers,
                broad_phase,
            ));
        }
    }
    buffers.uploaded = Some(scene.0.clone());
}
