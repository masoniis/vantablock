use bevy::render::render_graph::RenderLabel;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub enum VantablockNode {
    ShadowPass,
    OpaquePass,
    TransparentPass,
    BoundingBoxPass,
}
