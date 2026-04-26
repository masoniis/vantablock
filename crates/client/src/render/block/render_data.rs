use crate::render::block::texture_registry::TextureId;

/// Optimized hot block data required for meshing and rendering.
#[derive(Debug, Clone)]
pub struct BlockRenderData<T = TextureId> {
    pub textures: BlockFaceTextures<T>,
    pub is_transparent: bool,
}

/// The textures associated with each face of a particular block type.
#[derive(Debug, Clone)]
pub struct BlockFaceTextures<T> {
    pub top: T,
    pub bottom: T,
    pub front: T,
    pub back: T,
    pub right: T,
    pub left: T,
}

impl<T: Clone> BlockFaceTextures<T> {
    pub fn map<U, F>(self, mut f: F) -> BlockFaceTextures<U>
    where
        F: FnMut(T) -> U,
    {
        BlockFaceTextures {
            top: f(self.top.clone()),
            bottom: f(self.bottom.clone()),
            front: f(self.front.clone()),
            back: f(self.back.clone()),
            right: f(self.right.clone()),
            left: f(self.left.clone()),
        }
    }

    #[inline(always)]
    pub fn get(&self, face_index: usize) -> T {
        match face_index {
            0 => self.top.clone(),
            1 => self.bottom.clone(),
            2 => self.left.clone(),
            3 => self.right.clone(),
            4 => self.front.clone(),
            _ => self.back.clone(),
        }
    }
}
