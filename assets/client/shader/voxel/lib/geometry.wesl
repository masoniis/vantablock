// Bind group 0: the immutable geometry library, and logical handle resolution.
//
// Each arena lives in up to four slabs (see crates/engine/voxel/src/residency.rs). Page tables map
// a logical page to a slab address, the slab in the top two bits and the word below; a handle
// resolves to that plus its word offset. The `multi_slab` variant switches on the slab at every
// load; the single-slab variant reads slab 0 directly, since even a uniform branch per node fetch
// costs the DDA loop about 10% (gpu_scene_throughput on an M1 Pro).

import super::types::{Bvh4Node, GeometryMeta};

@group(0) @binding(0) var<storage, read> topology: array<u32>;
@group(0) @binding(1) var<storage, read> attributes: array<u32>;
// palette entries: linear albedo in rgb, emission in a
@group(0) @binding(2) var<storage, read> materials: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read> template_bvh: array<Bvh4Node>;
@group(0) @binding(4) var<uniform> geometry_meta: GeometryMeta;
@group(0) @binding(5) var<storage, read> topology_1: array<u32>;
@group(0) @binding(6) var<storage, read> topology_2: array<u32>;
@group(0) @binding(7) var<storage, read> topology_3: array<u32>;
@group(0) @binding(8) var<storage, read> attributes_1: array<u32>;
@group(0) @binding(9) var<storage, read> attributes_2: array<u32>;
@group(0) @binding(10) var<storage, read> attributes_3: array<u32>;
// packed LOD summaries (lod.wesl), indexed by a node header's lod_meta; deduplicated and small
// next to topology, so they live in one binding whatever the residency layout
@group(0) @binding(11) var<storage, read> lod_records: array<vec4<u32>>;

const WORD_OFFSET_MASK: u32 = 0xFFFFFFu;
const SLAB_SHIFT: u32 = 30u;
const SLAB_WORD_MASK: u32 = 0x3FFFFFFFu;

// Slab address of a logical topology page's first word.
fn topology_page_base(page: u32) -> u32 {
    return geometry_meta.topology_page_base[page >> 2u][page & 3u];
}

// Reads the topology word at a slab address.
@ if(multi_slab)
fn topology_load(address: u32) -> u32 {
    let i = address & SLAB_WORD_MASK;
    switch address >> SLAB_SHIFT {
        case 1u: {
            return topology_1[i];
        }
        case 2u: {
            return topology_2[i];
        }
        case 3u: {
            return topology_3[i];
        }
        default: {
            return topology[i];
        }
    }
}

@ if(!multi_slab)
fn topology_load(address: u32) -> u32 {
    return topology[address & SLAB_WORD_MASK];
}

// Resolves a logical attribute handle (8-bit page + 24-bit word offset) to a slab address.
fn attribute_address(attribute_handle: u32) -> u32 {
    let page = attribute_handle >> 24u;
    return geometry_meta.attribute_page_base[page >> 2u][page & 3u] + (attribute_handle & WORD_OFFSET_MASK);
}

@ if(multi_slab)
fn attribute_load(address: u32) -> u32 {
    let i = address & SLAB_WORD_MASK;
    switch address >> SLAB_SHIFT {
        case 1u: {
            return attributes_1[i];
        }
        case 2u: {
            return attributes_2[i];
        }
        case 3u: {
            return attributes_3[i];
        }
        default: {
            return attributes[i];
        }
    }
}

@ if(!multi_slab)
fn attribute_load(address: u32) -> u32 {
    return attributes[address & SLAB_WORD_MASK];
}

fn voxel_material(attribute_handle: u32) -> u32 {
    return attribute_load(attribute_address(attribute_handle)) & 0xFFFFu;
}

fn lod_record_load(index: u32) -> vec4<u32> {
    return lod_records[index];
}
