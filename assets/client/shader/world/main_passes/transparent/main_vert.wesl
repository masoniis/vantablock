import package::world::main_passes::{
  lib::camera_uniform::{camera},
  lib::geometry_bindings::{faces, chunks, environment},
};
import package::world::lib::{
  vertex_pulling::{
    pull_global_vertex_position,
    get_voxel_uv,
    get_voxel_normal
  },
  face_unpacking::{unpack_face}
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) tex_coords: vec2<f32>,
    @location(3) @interpolate(flat) texture_index: u32,
    @location(4) world_pos: vec3<f32>,
    @location(5) clip_pos_for_fog: vec4<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_idx: u32,
    @builtin(instance_index) instance_idx: u32,
) -> VertexOutput {
    let chunk = chunks[instance_idx];

    // INFO: ---------------------------------------------------
    //         determine vertex pos form storage buffers
    // ---------------------------------------------------------

    // unpack face bytes
    let local_face_idx = vertex_idx / 6u; // unsigned div truncates
    let global_face_idx = chunk.start_index + local_face_idx;
    let packed_face = faces[global_face_idx];
    let face = unpack_face(packed_face);

    // calculate world position of this vertex for this face
    let vert_idx_in_quad = vertex_idx % 6u;
    let world_pos_vec3 = pull_global_vertex_position(
        vec3<u32>(face.position),
        face.normal_index,
        vert_idx_in_quad,
        chunk.world_pos
    );

    // INFO: ---------------------------
    //         basic wobble wave
    // ---------------------------------

    var final_pos = world_pos_vec3;
    let wobble = sin((world_pos_vec3.x + world_pos_vec3.z) * 23.0 + environment.time) * 0.05;
    final_pos.y += wobble - 0.05; // offset to ensure vertices below voxels next to it

    // INFO: -------------------------
    //         output for frag
    // -------------------------------

    var out: VertexOutput;

    out.world_pos = final_pos;
    out.clip_position = camera.view_proj * vec4<f32>(final_pos, 1.0);
    out.clip_pos_for_fog = out.clip_position;

    out.normal = get_voxel_normal(face.normal_index);
    out.color = vec3<f32>(1.0, 1.0, 1.0);
    out.tex_coords = get_voxel_uv(vert_idx_in_quad);
    out.texture_index = face.texture_id;

    return out;
}

