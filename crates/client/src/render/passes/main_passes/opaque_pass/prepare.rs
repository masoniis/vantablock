use crate::prelude::*;

/// A system that no longer needs to do anything because mesh uploads are
/// handled once per frame by the shared `MeshesToUploadQueue` in the
/// transparent pass (or any pass that runs first).
///
/// Since the SSBO and `RenderMeshStorageResource` are shared, processing the
/// queue once is sufficient.
#[instrument(skip_all)]
pub fn prepare_opaque_meshes_system() {}

/// Removed in favor of handling removals in the main prepare system.
pub fn delete_gpu_buffers_system() {}
