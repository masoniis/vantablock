import package::world::lib::face::{unpack_face, UnpackedFace, UnpackedGeometry, unpack_face_geometry};

// INFO: -----------------------
//         lookup tables
// -----------------------------

/// A lookup table for the 6 potential UVs of a quad.
const QUAD_UVS: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    vec2<f32>(0.0, 1.0), // bottom-left (inverted Y in UV)
    vec2<f32>(1.0, 1.0), // bottom-right
    vec2<f32>(1.0, 0.0), // top-right
    vec2<f32>(1.0, 0.0), // top-right
    vec2<f32>(0.0, 0.0), // top-left
    vec2<f32>(0.0, 1.0)  // bottom-left
);

/// A lookup table for the 6 potential normal vectors of a voxel.
const CUBE_NORMALS: array<vec3<f32>, 6> = array<vec3<f32>, 6>(
    vec3<f32>(0.0, 1.0, 0.0),  // top face
    vec3<f32>(0.0, -1.0, 0.0), // bottom
    vec3<f32>(1.0, 0.0, 0.0),  // right
    vec3<f32>(-1.0, 0.0, 0.0), // left
    vec3<f32>(0.0, 0.0, 1.0),  // front
    vec3<f32>(0.0, 0.0, -1.0), // back
);

/// A lookup table for the relative position of the 8 vertices of a voxel.
const CUBE_VERTICES: array<vec3<f32>, 8> = array<vec3<f32>, 8>(
    vec3<f32>(0.0, 0.0, 0.0), vec3<f32>(1.0, 0.0, 0.0),
    vec3<f32>(1.0, 1.0, 0.0), vec3<f32>(0.0, 1.0, 0.0),
    vec3<f32>(0.0, 0.0, 1.0), vec3<f32>(1.0, 0.0, 1.0),
    vec3<f32>(1.0, 1.0, 1.0), vec3<f32>(0.0, 1.0, 1.0),
);

/// A lookup table for the indices necessary to draw a cube based on the vertices
/// defined in the `CUBE_VERTICES` lookup table seen above.
const CUBE_INDICES: array<u32, 36> = array<u32, 36>(
    7, 6, 2, 2, 3, 7, // top face (front to back winding)
    0, 1, 5, 5, 4, 0, // bottom   (back to front winding)
    5, 1, 2, 2, 6, 5, // right
    0, 4, 7, 7, 3, 0, // left
    4, 5, 6, 6, 7, 4, // front
    1, 0, 3, 3, 2, 1, // back
);

/// Maps the integer AO level (0-3) to a brightness float (0.0-1.0).
const AO_VALUES: array<f32, 4> = array<f32, 4>(
    1.0, // full lit
    0.8, // slight
    0.5, // darker
    0.2, // darkest
);

/// Maps the 6 vertices of a quad (2 triangles) to the 4 corners of the face.
///
/// Indices: 0=BottomLeft, 1=TopLeft, 2=BottomRight, 3=TopRight
const QUAD_VERTEX_TO_CORNER: array<u32, 6> = array<u32, 6>(
    0u, 1u, 2u, // triangle 1
    2u, 3u, 0u  // triangle 2
);


// INFO: ---------------
//         utils
// ---------------------

/// Calculates the final world position of a specific vertex.
fn pull_global_vertex_position(
    face_pos: vec3<u32>,
    normal_idx: u32,
    vert_idx_in_quad: u32,
    chunk_world_pos: vec3<f32>
) -> vec3<f32> {
    let lut_index = (normal_idx * 6u) + vert_idx_in_quad;
    let corner_index = CUBE_INDICES[lut_index];
    let corner_offset = CUBE_VERTICES[corner_index];

    // local position in chunk (0..32) + corner offset (0..1) + chunk world pos
    return vec3<f32>(face_pos) + corner_offset + chunk_world_pos;
}

/// Gets the UVs for a voxel given the vertex indice in the face quad.
fn get_voxel_uv(vert_idx_in_quad: u32) -> vec2<f32> {
    return QUAD_UVS[vert_idx_in_quad];
}

/// Gets the UVs for a voxel given the vertex indice in the face quad.
fn get_ao_color(ao_vec: vec4<f32>, vert_idx_in_quad: u32) -> vec3<f32> {
    let corner_idx = QUAD_VERTEX_TO_CORNER[vert_idx_in_quad];
    let raw_level = u32(ao_vec[corner_idx]);
    let factor = AO_VALUES[raw_level];
    return vec3(factor, factor, factor);
}

/// Gets the normal vector given the normal index from the unpacked vertex.
fn get_voxel_normal(normal_idx: u32) -> vec3<f32> {
    return CUBE_NORMALS[normal_idx];
}
