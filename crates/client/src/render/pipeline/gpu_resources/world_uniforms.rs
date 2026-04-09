use crate::prelude::*;
use crate::render::chunk::meshing::packed_face::PackedFace;
use bevy::ecs::prelude::*;
use bevy::render::render_resource::{
    BindGroup, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingResource, BindingType, Buffer, BufferBindingType, BufferDescriptor, BufferUsages,
    CommandEncoderDescriptor, ShaderStages,
};
use bevy::render::renderer::{RenderDevice, RenderQueue};
use offset_allocator::{Allocation, Allocator};

/// The virtual address space size the allocator believes it has.
const VIRTUAL_ADDRESS_SPACE: u32 = 2 * 1024 * 1024 * 1024; // (2 GB)

// Initial physical size set at 1MB, let resizing handle the rest
const INITIAL_PHYSICAL_SIZE: u64 = 1024 * 1024; // (1 MB)

/// The max number of chunks for storing metadata
const MAX_CHUNKS: u64 = 10_000;

// INFO: --------------------
//         data types
// --------------------------

/// A representation of raw chunk data used in the chunk metadata storage buffer.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ChunkRenderData {
    pub world_pos: [f32; 3],
    pub start_index: u32,
}

/// A voxel mesh handle holding the GPU allocation and index.
#[derive(Clone, Copy)]
pub struct VoxelMesh {
    /// Handle for the geometry (face) buffer
    pub geometry_allocation: Allocation,
    /// Handle for metadata buffer (instance index)
    pub slot_index: u32,
    /// Number of faces to draw
    pub face_count: u32,
}

// INFO: --------------------------
//         internal helpers
// --------------------------------

/// A simple Free-List allocator for fixed-size slots (0..N).
struct SlotAllocator {
    free_indices: Vec<u32>,
    next_index: u32,
    capacity: u32,
}

impl SlotAllocator {
    fn new(capacity: u32) -> Self {
        Self {
            free_indices: Vec::new(),
            next_index: 0,
            capacity,
        }
    }

    fn allocate(&mut self) -> Option<u32> {
        // recycle used slot if possible
        if let Some(idx) = self.free_indices.pop() {
            return Some(idx);
        }
        // else create a new one
        if self.next_index < self.capacity {
            let idx = self.next_index;
            self.next_index += 1;
            return Some(idx);
        }
        // or full
        None
    }

    fn free(&mut self, index: u32) {
        self.free_indices.push(index);
    }
}

// INFO: ---------------------
//         the manager
// ---------------------------

/// The view bind group layout resource shared by all camera-centric render passes.
#[derive(Resource, Clone)]
pub struct ChunkStorageBindGroupLayout {
    pub layout: BindGroupLayout,
    pub descriptor: BindGroupLayoutDescriptor,
}

impl FromWorld for ChunkStorageBindGroupLayout {
    #[instrument(skip_all)]
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();

        let descriptor = BindGroupLayoutDescriptor {
            label: "Chunk Storage Bind Group Layout".into(),
            entries: vec![
                BindGroupLayoutEntry {
                    binding: 0, // chunk metadata
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1, // faces
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        };

        let layout =
            device.create_bind_group_layout(descriptor.label.as_ref(), &descriptor.entries);

        Self { layout, descriptor }
    }
}

#[derive(Resource)]
pub struct ChunkStorageManager {
    pub bind_group: BindGroup,
    pub layout: ChunkStorageBindGroupLayout,

    // buffers
    meta_buffer: Buffer,
    face_buffer: Buffer,

    // allocators for buffers
    geometry_allocator: Allocator,
    slot_allocator: SlotAllocator,
}

impl FromWorld for ChunkStorageManager {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();
        let layout = world.resource::<ChunkStorageBindGroupLayout>();

        let face_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Global Voxel SSBO"),
            size: INITIAL_PHYSICAL_SIZE,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let meta_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Global Meta SSBO"),
            size: MAX_CHUNKS * std::mem::size_of::<ChunkRenderData>() as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = device.create_bind_group(
            Some("Global Voxel Bind Group"),
            &layout.layout,
            &[
                BindGroupEntry {
                    binding: 0, // chunk metadata
                    resource: meta_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1, // chunk faces
                    resource: face_buffer.as_entire_binding(),
                },
            ],
        );

        Self {
            face_buffer,
            meta_buffer,
            bind_group,
            layout: layout.clone(),
            geometry_allocator: Allocator::new(VIRTUAL_ADDRESS_SPACE),
            slot_allocator: SlotAllocator::new(MAX_CHUNKS as u32),
        }
    }
}

impl ChunkStorageManager {
    pub fn meta_buffer_binding(&self) -> BindingResource<'_> {
        self.meta_buffer.as_entire_binding()
    }

    /// Internal helper to resize the geometry buffer when virtual offsets exceed physical capacity.
    fn ensure_capacity(
        &mut self,
        device: &RenderDevice,
        queue: &RenderQueue,
        layout: &ChunkStorageBindGroupLayout,
        required_end_byte: u64,
    ) {
        let current_size = self.face_buffer.size();

        // if space permits already, we are good
        if required_end_byte <= current_size {
            return;
        }

        // new size calc
        let mut new_size = current_size * 2;
        while new_size < required_end_byte {
            new_size *= 2;
        }

        // if exceed hardware limit, panic for now
        let limits = device.limits();
        let max_buffer_size = limits.max_storage_buffer_binding_size as u64;
        if new_size > max_buffer_size {
            new_size = max_buffer_size;
            if required_end_byte > max_buffer_size {
                panic!(
                    "Voxel geometry exceeded GPU hardware limit of {} bytes!",
                    max_buffer_size
                );
            }
        }

        debug!(
            target : "gpu_memory",
            "Resizing voxel geometry buffer to {} MB",
            new_size / 1024 / 1024
        );

        // new buffer
        let new_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Global Voxel SSBO (Resized)"),
            size: new_size,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // get raw wgpu_device for the command encoder for manual resize
        let mut encoder = device
            .wgpu_device()
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Resize Copy Encoder"),
            });
        encoder.copy_buffer_to_buffer(&self.face_buffer, 0, &new_buffer, 0, current_size);

        queue.submit([encoder.finish()]);

        // replace buffer and rebuild bind group
        self.face_buffer = new_buffer;
        self.bind_group = device.create_bind_group(
            Some("Global Voxel Bind Group"),
            &layout.layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: self.meta_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: self.face_buffer.as_entire_binding(),
                },
            ],
        );
    }

    /// Upload a chunk to the GPU buffer.
    pub fn allocate_chunk(
        &mut self,
        device: &RenderDevice,
        queue: &RenderQueue,
        layout: &ChunkStorageBindGroupLayout,
        faces: &[PackedFace],
        world_pos: [f32; 3],
    ) -> Option<VoxelMesh> {
        if faces.is_empty() {
            return None;
        }

        // allocate faces (variable in size)
        let size_bytes = std::mem::size_of_val(faces) as u32;

        // check if allocation offset is outside physical address space and resize if necessary
        let geometry_allocation = self.geometry_allocator.allocate(size_bytes)?;
        let required_end = (geometry_allocation.offset + size_bytes) as u64;
        self.ensure_capacity(device, queue, layout, required_end);

        // allocate slot (fixed size)
        let slot_index = match self.slot_allocator.allocate() {
            Some(s) => s,
            None => {
                self.geometry_allocator.free(geometry_allocation);
                return None;
            }
        };

        queue.write_buffer(
            &self.face_buffer,
            geometry_allocation.offset as u64,
            bytemuck::cast_slice(faces),
        );

        let start_index = geometry_allocation.offset / 4;

        let meta_data = ChunkRenderData {
            world_pos,
            start_index,
        };

        let meta_offset = slot_index as u64 * std::mem::size_of::<ChunkRenderData>() as u64;
        queue.write_buffer(
            &self.meta_buffer,
            meta_offset,
            bytemuck::bytes_of(&meta_data),
        );

        Some(VoxelMesh {
            geometry_allocation,
            slot_index,
            face_count: faces.len() as u32,
        })
    }

    /// Frees all resources associated with the chunk handle.
    pub fn free_chunk(&mut self, mesh: VoxelMesh) {
        self.geometry_allocator.free(mesh.geometry_allocation);
        self.slot_allocator.free(mesh.slot_index);
    }
}
