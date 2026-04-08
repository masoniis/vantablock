import package::world::lib::face_unpacking::{unpack_face_geometry};
import package::world::lib::vertex_pulling::{pull_global_vertex_position};
import super::lib::sun_camera_uniform::{SunCameraUniform};

struct ChunkRenderData {
    world_pos: vec3<f32>,
    start_index: u32,
};

@group(1) @binding(0)
var<storage, read> chunks: array<ChunkRenderData>;

@group(1) @binding(1)
var<storage, read> faces: array<u32>;

@group(0) @binding(0)
var<uniform> shadow_camera: SunCameraUniform;

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_idx: u32,
    @builtin(instance_index) instance_idx: u32
) -> @builtin(position) vec4<f32> {
    let chunk: ChunkRenderData = chunks[instance_idx];

    // INFO: ---------------------------------------------------
    //         determine vertex pos from storage buffers
    // ---------------------------------------------------------

    let local_face_idx = vertex_idx / 6u;
    let global_face_idx = chunk.start_index + local_face_idx;

    let packed_face = faces[global_face_idx];
    let geometry = unpack_face_geometry(packed_face);

    let vert_idx_in_quad = vertex_idx % 6u;

    let world_pos_vec3 = pull_global_vertex_position(
        vec3<u32>(geometry.position),
        geometry.normal_index,
        vert_idx_in_quad,
        chunk.world_pos
    );

    // INFO: ----------------
    //         output
    // ----------------------

    return shadow_camera.view_proj * vec4<f32>(world_pos_vec3, 1.0);
}
