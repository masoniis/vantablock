// Bind group 0: the immutable geometry library, and logical handle resolution.

import super::types::{Bvh4Node, GeometryMeta};

@group(0) @binding(0) var<storage, read> topology: array<u32>;
@group(0) @binding(1) var<storage, read> attributes: array<u32>;
// palette entries: linear albedo in rgb, emission in a
@group(0) @binding(2) var<storage, read> materials: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read> template_bvh: array<Bvh4Node>;
@group(0) @binding(4) var<uniform> geometry_meta: GeometryMeta;

const WORD_OFFSET_MASK: u32 = 0xFFFFFFu;

// Slab word base of a logical topology page.
fn topology_page_base(page: u32) -> u32 {
    return geometry_meta.topology_page_base[page >> 2u][page & 3u];
}

// Resolves a logical attribute handle (8-bit page + 24-bit word offset) to a slab index.
fn attribute_address(attribute_handle: u32) -> u32 {
    let page = attribute_handle >> 24u;
    return geometry_meta.attribute_page_base[page >> 2u][page & 3u] + (attribute_handle & WORD_OFFSET_MASK);
}

fn voxel_material(attribute_handle: u32) -> u32 {
    return attributes[attribute_address(attribute_handle)] & 0xFFFFu;
}
