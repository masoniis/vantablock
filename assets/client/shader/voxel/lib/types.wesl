// GPU data layouts, mirrored field for field in crates/game/client/src/render/gpu/layout.rs.

struct Bvh4Node {
    min_x: vec4<f32>,
    min_y: vec4<f32>,
    min_z: vec4<f32>,
    max_x: vec4<f32>,
    max_y: vec4<f32>,
    max_z: vec4<f32>,
    // node index, BVH_LEAF_BIT | primitive, or BVH_EMPTY
    children: vec4<u32>,
    pad: vec4<u32>,
};

struct PoseFrame {
    world_to_local: array<vec4<f32>, 3>,
    local_to_world: array<vec4<f32>, 3>,
    prev_local_to_world: array<vec4<f32>, 3>,
    bounds_min: vec3<f32>,
    root_node: u32,
    bounds_max: vec3<f32>,
    root_page_base: u32,
    levels: u32,
    flags: u32,
    palette_base: u32,
    instance: u32,
};

struct InstanceRecord {
    world_to_root: array<vec4<f32>, 3>,
    mini_bvh_root: u32,
    slot_base: u32,
    group_count: u32,
    flags: u32,
};

struct SceneUniform {
    frame_count: u32,
    instance_count: u32,
    bvh_node_count: u32,
    mode: u32,
};

struct GeometryMeta {
    topology_page_base: array<vec4<u32>, 64>,
    attribute_page_base: array<vec4<u32>, 64>,
};

struct ViewUniform {
    world_from_clip: mat4x4<f32>,
    clip_from_world: mat4x4<f32>,
    camera_position: vec3<f32>,
    debug_view: u32,
    // size of the traced ray grid (the visibility buffer)
    viewport: vec2<f32>,
    flags: u32,
    // ray grid pixels per view target pixel (the render scale)
    target_to_grid: f32,
};

struct Environment {
    light_direction: vec3<f32>,
    ambient: f32,
    light_color: vec3<f32>,
    time: f32,
    zenith: vec3<f32>,
    pad0: f32,
    horizon: vec3<f32>,
    pad1: f32,
    sun_direction: vec3<f32>,
    pad2: f32,
    sun_disk: vec3<f32>,
    pad3: f32,
    moon_direction: vec3<f32>,
    pad4: f32,
};

const FRAME_SINGLE_PAGE: u32 = 1u;
const FRAME_OCTREE: u32 = 2u;
const INSTANCE_USES_TEMPLATE: u32 = 1u;
const NO_MINI_BVH: u32 = 0xFFFFFFFFu;
const VISIBILITY_MISS: u32 = 0xFFFFFFFFu;
const MODE_TEMPLATE_MINI_BVH: u32 = 0u;
const MODE_FLATTENED_GROUPS: u32 = 1u;
const VIEW_TONEMAP_IN_SHADER: u32 = 1u;
