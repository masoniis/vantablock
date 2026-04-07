use bevy::render::render_graph::RenderLabel;

#[derive(RenderLabel, Debug, Hash, PartialEq, Eq, Clone)]
pub enum VantablockNode {
    ShadowPass,
    OpaquePass,
    TransparentPass,
    BoundingBoxPass,
}
