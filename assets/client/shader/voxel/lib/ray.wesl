// Ray helpers. Mirrors crates/voxel/src/math.rs.

const MIN_DIRECTION_COMPONENT: f32 = 1e-8;

// Replaces near-zero direction components (keeping their sign) so slab tests never compute 0 * inf.
fn sanitize_direction(d: vec3<f32>) -> vec3<f32> {
    let small = abs(d) < vec3<f32>(MIN_DIRECTION_COMPONENT);
    let nudged = select(
        vec3<f32>(MIN_DIRECTION_COMPONENT),
        vec3<f32>(-MIN_DIRECTION_COMPONENT),
        d < vec3<f32>(0.0)
    );
    return select(d, nudged, small);
}

// The slab test: (t_enter, t_exit); the ray misses when x > y.
fn slab(lo: vec3<f32>, hi: vec3<f32>, o: vec3<f32>, inv: vec3<f32>) -> vec2<f32> {
    let t0 = (lo - o) * inv;
    let t1 = (hi - o) * inv;
    let near = min(t0, t1);
    let far = max(t0, t1);
    return vec2<f32>(max(max(near.x, near.y), near.z), min(min(far.x, far.y), far.z));
}

fn transform_point(m: array<vec4<f32>, 3>, p: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(dot(m[0].xyz, p) + m[0].w, dot(m[1].xyz, p) + m[1].w, dot(m[2].xyz, p) + m[2].w);
}

fn transform_vector(m: array<vec4<f32>, 3>, v: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(dot(m[0].xyz, v), dot(m[1].xyz, v), dot(m[2].xyz, v));
}
