//! The derived geometry library: every baked voxel root shares one set of arenas.

use crate::{
    arena::{ArenaError, Attributes, Handle, Topology, WordArena},
    handle::{ArenaHandle, MAX_PAGE_WORDS},
    hierarchy::tree64::{self, BakeStats, Tree64Root},
    lod::LodTable,
    module::VoxelModule,
};

/// Read access to baked geometry, implemented by anything that can resolve logical handles.
pub trait GeometrySource {
    fn topology_word(&self, handle: ArenaHandle) -> u32;
    fn attribute_word(&self, handle: ArenaHandle) -> u32;
}

/// The topology, attribute, and LOD stores shared by all baked roots.
pub struct GeometryArenas {
    pub topology: WordArena<Topology>,
    pub attributes: WordArena<Attributes>,
    pub lod: LodTable,
}

impl Default for GeometryArenas {
    fn default() -> Self {
        Self::with_page_words(MAX_PAGE_WORDS)
    }
}

impl GeometryArenas {
    /// Arenas with a custom page size. Small pages force multi-page layouts in tests.
    pub fn with_page_words(page_words: u32) -> Self {
        Self {
            topology: WordArena::new(page_words),
            attributes: WordArena::new(page_words),
            lod: LodTable::default(),
        }
    }

    /// Bakes a module into a 64-tree root inside these arenas, sharing identical blocks with
    /// everything baked before it.
    pub fn bake_tree64(
        &mut self,
        module: &VoxelModule,
    ) -> Result<(Tree64Root, BakeStats), ArenaError> {
        tree64::bake(self, module)
    }

    /// Frees the deduplication indices once no more bakes will happen.
    pub fn finish(&mut self) {
        self.topology.finish_dedup();
        self.attributes.finish_dedup();
        self.lod.finish_dedup();
    }

    pub fn total_bytes(&self) -> u64 {
        (self.topology.used_words() + self.attributes.used_words()) * 4
            + self.lod.records().len() as u64 * 16
    }
}

impl GeometrySource for GeometryArenas {
    fn topology_word(&self, handle: ArenaHandle) -> u32 {
        self.topology.word(Handle::from_untyped(handle))
    }

    fn attribute_word(&self, handle: ArenaHandle) -> u32 {
        self.attributes.word(Handle::from_untyped(handle))
    }
}
