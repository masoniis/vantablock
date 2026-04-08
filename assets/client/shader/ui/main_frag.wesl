import package::ui::common::{material, Material};

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return material.color;
}