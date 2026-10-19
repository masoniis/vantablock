//! GPU adapter probing.
//!
//! Bevy is configured to request the adapter's own limits and features rather than the WebGPU
//! defaults (clearing `constrained_limits` alone would not do that). After the device exists,
//! its *effective* limits are logged and captured, and residency planning uses those, never an
//! assumed 512 MiB or 1 GiB binding.

use bevy::{
    prelude::*,
    render::{
        renderer::{RenderAdapterInfo, RenderDevice},
        settings::{Backends, WgpuFeatures, WgpuSettings, WgpuSettingsPriority},
    },
};
use voxel::residency::ResidencyLimits;

/// Storage buffers bound by the voxel passes: geometry (four slabs each of topology and
/// attributes, materials, template mini-BVHs) and scene (BVH, frames, instances, slot table).
pub const REQUIRED_STORAGE_BUFFERS_PER_STAGE: u32 = 14;
/// Storage textures written by the primary visibility pass.
pub const REQUIRED_STORAGE_TEXTURES_PER_STAGE: u32 = 4;

/// Renderer creation settings that request the adapter's real limits and features.
pub fn wgpu_settings() -> WgpuSettings {
    WgpuSettings {
        priority: WgpuSettingsPriority::Functionality,
        ..default()
    }
}

/// Whether the hardware broad phase can run: the device has ray queries, on a backend whose
/// shader translation supports them.
///
/// naga's Metal backend (as of wgpu 29) lowers `rayQueryProceed` to a flag that only
/// `rayQueryTerminate` clears, over a single closest-hit `intersect`. It never steps to the next
/// candidate, so the broad phase's `while rayQueryProceed(..)` loop never ends and the command
/// buffer never completes. Vulkan and DX12 translate the candidate loop faithfully.
pub fn ray_query_supported(backend: impl Into<Backends>, features: WgpuFeatures) -> bool {
    features.contains(WgpuFeatures::EXPERIMENTAL_RAY_QUERY) && backend.into() != Backends::METAL
}

/// What the created device can actually do.
#[derive(Resource, Clone, Debug)]
pub struct GpuCapabilities {
    pub adapter_name: String,
    pub backend: String,
    pub residency: ResidencyLimits,
    pub max_storage_buffers_per_stage: u32,
    pub max_storage_textures_per_stage: u32,
    /// Optional accelerators. None are required; each must be capability-checked before use.
    pub ray_query: bool,
    pub timestamp_queries: bool,
    pub subgroups: bool,
    pub shader_f16: bool,
}

impl GpuCapabilities {
    pub fn probe(device: &RenderDevice, adapter: &RenderAdapterInfo) -> Self {
        let limits = device.limits();
        let features = device.features();
        Self {
            adapter_name: adapter.name.clone(),
            backend: format!("{:?}", adapter.backend),
            residency: ResidencyLimits {
                max_storage_buffer_binding_size: limits.max_storage_buffer_binding_size,
                max_buffer_size: limits.max_buffer_size,
            },
            max_storage_buffers_per_stage: limits.max_storage_buffers_per_shader_stage,
            max_storage_textures_per_stage: limits.max_storage_textures_per_shader_stage,
            ray_query: ray_query_supported(adapter.backend, features),
            timestamp_queries: features.contains(WgpuFeatures::TIMESTAMP_QUERY),
            subgroups: features.contains(WgpuFeatures::SUBGROUP),
            shader_f16: features.contains(WgpuFeatures::SHADER_F16),
        }
    }

    /// Requirements the voxel passes can't run without.
    pub fn missing_requirements(&self) -> Vec<String> {
        let mut missing = Vec::new();
        if self.max_storage_buffers_per_stage < REQUIRED_STORAGE_BUFFERS_PER_STAGE {
            missing.push(format!(
                "{} storage buffers per stage (device allows {})",
                REQUIRED_STORAGE_BUFFERS_PER_STAGE, self.max_storage_buffers_per_stage
            ));
        }
        if self.max_storage_textures_per_stage < REQUIRED_STORAGE_TEXTURES_PER_STAGE {
            missing.push(format!(
                "{} storage textures per stage (device allows {})",
                REQUIRED_STORAGE_TEXTURES_PER_STAGE, self.max_storage_textures_per_stage
            ));
        }
        missing
    }

    pub fn log(&self) {
        info!(
            "GPU: {} ({}), storage binding {} MiB, buffer {} MiB, {} storage buffers / {} storage textures per stage",
            self.adapter_name,
            self.backend,
            self.residency.max_storage_buffer_binding_size >> 20,
            self.residency.max_buffer_size >> 20,
            self.max_storage_buffers_per_stage,
            self.max_storage_textures_per_stage
        );
        info!(
            "GPU optional features: ray query {}, timestamps {}, subgroups {}, f16 {}",
            self.ray_query, self.timestamp_queries, self.subgroups, self.shader_f16
        );
        for missing in self.missing_requirements() {
            error!("GPU lacks a voxel renderer requirement: {}", missing);
        }
    }
}
