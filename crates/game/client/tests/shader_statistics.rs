//! Compiler statistics for the voxel compute kernels: registers, spills, and shared memory, as the
//! Vulkan driver reports them through `VK_KHR_pipeline_executable_properties`.
//!
//! wgpu exposes none of this, so the WGSL goes through naga's SPIR-V backend with the options
//! wgpu's Vulkan backend uses (bounds checks, loop bounding, robust buffer access), and a bare
//! Vulkan device compiles it with statistics capture. The numbers are the driver's own: NVIDIA
//! reports register counts, local (spill) memory, and shared memory; other vendors differ.
//!
//! Not a correctness test; run with `just shader-stats` on a Vulkan adapter. Set
//! `WGPU_ADAPTER_NAME` to pick an adapter by name.

use ash::vk;
use std::ffi::{CStr, CString};
use wesl_core::include_wesl;

/// `(label, WGSL, compute entry point)`.
const KERNELS: &[(&str, &str, &str)] = &[
    (
        "primary_visibility",
        include_wesl!("primary_visibility"),
        "main",
    ),
    (
        "primary_visibility (multi slab)",
        include_wesl!("primary_visibility_multi_slab"),
        "main",
    ),
    (
        "primary_visibility (ray query)",
        include_wesl!("primary_visibility_ray_query"),
        "main",
    ),
    (
        "primary_visibility (footprint lod)",
        include_wesl!("primary_visibility_footprint_lod"),
        "main",
    ),
    (
        "primary_visibility (footprint lod, ray query)",
        include_wesl!("primary_visibility_footprint_lod_ray_query"),
        "main",
    ),
    (
        "scene_rays",
        include_wesl!("voxel_conformance"),
        "scene_rays",
    ),
    (
        "scene_rays (stackless)",
        include_wesl!("voxel_conformance_stackless"),
        "scene_rays",
    ),
    (
        "scene_rays (multi slab)",
        include_wesl!("voxel_conformance_multi_slab"),
        "scene_rays",
    ),
    (
        "scene_rays (ray query)",
        include_wesl!("voxel_conformance_ray_query"),
        "scene_rays",
    ),
    // the bare 64-tree traversal, exact and with footprint LOD filtering
    ("rays", include_wesl!("voxel_conformance"), "rays"),
    (
        "filtered_rays (footprint lod)",
        include_wesl!("voxel_conformance_footprint_lod"),
        "filtered_rays",
    ),
    // whole-scene traversal with footprint LOD filtering
    (
        "filtered_scene_rays (footprint lod)",
        include_wesl!("voxel_conformance_footprint_lod"),
        "filtered_scene_rays",
    ),
    (
        "filtered_scene_rays (footprint lod, ray query)",
        include_wesl!("voxel_conformance_footprint_lod_ray_query"),
        "filtered_scene_rays",
    ),
];

/// SPIR-V for one WGSL module, with the naga options wgpu's Vulkan backend uses on a device with
/// robust buffer and image access (`wgpu-hal`'s `Adapter::open`, default runtime checks).
fn spirv(label: &str, source: &str) -> Vec<u32> {
    let module = naga::front::wgsl::parse_str(source)
        .unwrap_or_else(|e| panic!("{label} failed to parse:\n{}", e.emit_to_string(source)));
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .unwrap_or_else(|e| panic!("{label} failed validation:\n{}", e.emit_to_string(source)));
    let options = naga::back::spv::Options {
        lang_version: (1, 6),
        bounds_check_policies: naga::proc::BoundsCheckPolicies {
            index: naga::proc::BoundsCheckPolicy::Restrict,
            buffer: naga::proc::BoundsCheckPolicy::Unchecked,
            image_load: naga::proc::BoundsCheckPolicy::Unchecked,
            binding_array: naga::proc::BoundsCheckPolicy::Unchecked,
        },
        force_loop_bounding: true,
        ray_query_initialization_tracking: true,
        ..Default::default()
    };
    naga::back::spv::write_vec(&module, &info, &options, None)
        .unwrap_or_else(|e| panic!("{label} failed to write SPIR-V: {e}"))
}

/// Every resource binding a module declares, as `(group, binding, descriptor type)`.
fn bindings(module: &naga::Module) -> Vec<(u32, u32, vk::DescriptorType)> {
    module
        .global_variables
        .iter()
        .filter_map(|(_, var)| {
            let binding = var.binding.as_ref()?;
            let kind = match var.space {
                naga::AddressSpace::Uniform => vk::DescriptorType::UNIFORM_BUFFER,
                naga::AddressSpace::Storage { .. } => vk::DescriptorType::STORAGE_BUFFER,
                naga::AddressSpace::Handle => match module.types[var.ty].inner {
                    naga::TypeInner::Image {
                        class: naga::ImageClass::Storage { .. },
                        ..
                    } => vk::DescriptorType::STORAGE_IMAGE,
                    naga::TypeInner::Image { .. } => vk::DescriptorType::SAMPLED_IMAGE,
                    naga::TypeInner::Sampler { .. } => vk::DescriptorType::SAMPLER,
                    naga::TypeInner::AccelerationStructure { .. } => {
                        vk::DescriptorType::ACCELERATION_STRUCTURE_KHR
                    }
                    ref other => panic!("unexpected handle type {other:?}"),
                },
                other => panic!("unexpected bound address space {other:?}"),
            };
            Some((binding.group, binding.binding, kind))
        })
        .collect()
}

struct Driver {
    _entry: ash::Entry,
    instance: ash::Instance,
    device: ash::Device,
    executables: ash::khr::pipeline_executable_properties::Device,
    adapter: String,
    ray_query: bool,
}

impl Driver {
    fn new() -> Option<Self> {
        // SAFETY: loading the system Vulkan loader; nothing else is called if it is missing.
        let entry = match unsafe { ash::Entry::load() } {
            Ok(entry) => entry,
            Err(e) => {
                eprintln!("skipping shader statistics: no Vulkan loader ({e})");
                return None;
            }
        };
        let app = vk::ApplicationInfo::default().api_version(vk::API_VERSION_1_3);
        // SAFETY: plain instance creation with valid create info.
        let instance = unsafe {
            entry.create_instance(
                &vk::InstanceCreateInfo::default().application_info(&app),
                None,
            )
        }
        .ok()?;

        let wanted = std::env::var("WGPU_ADAPTER_NAME")
            .ok()
            .map(|n| n.to_lowercase());
        // SAFETY: queries on a live instance.
        let (physical, adapter, extensions) = unsafe { instance.enumerate_physical_devices() }
            .ok()?
            .into_iter()
            .filter_map(|pd| {
                // SAFETY: as above.
                let props = unsafe { instance.get_physical_device_properties(pd) };
                let name = props
                    .device_name_as_c_str()
                    .ok()?
                    .to_string_lossy()
                    .into_owned();
                let extensions: Vec<CString> =
                    unsafe { instance.enumerate_device_extension_properties(pd) }
                        .ok()?
                        .iter()
                        .filter_map(|e| e.extension_name_as_c_str().ok().map(CStr::to_owned))
                        .collect();
                let rank = match &wanted {
                    Some(w) if !name.to_lowercase().contains(w) => return None,
                    _ if props.device_type == vk::PhysicalDeviceType::DISCRETE_GPU => 0,
                    _ => 1,
                };
                Some((rank, pd, name, extensions))
            })
            .min_by_key(|(rank, ..)| *rank)
            .map(|(_, pd, name, extensions)| (pd, name, extensions))?;
        let has = |name: &CStr| extensions.iter().any(|e| e.as_c_str() == name);
        if !has(ash::khr::pipeline_executable_properties::NAME) {
            eprintln!(
                "skipping shader statistics: {adapter} lacks VK_KHR_pipeline_executable_properties"
            );
            return None;
        }
        let ray_query = has(ash::khr::ray_query::NAME)
            && has(ash::khr::acceleration_structure::NAME)
            && has(ash::khr::deferred_host_operations::NAME);

        // SAFETY: queries on a live instance.
        let family = unsafe { instance.get_physical_device_queue_family_properties(physical) }
            .iter()
            .position(|f| f.queue_flags.contains(vk::QueueFlags::COMPUTE))?
            as u32;
        let priorities = [1.0];
        let queues = [vk::DeviceQueueCreateInfo::default()
            .queue_family_index(family)
            .queue_priorities(&priorities)];

        let mut names = vec![
            ash::khr::pipeline_executable_properties::NAME.as_ptr(),
            ash::ext::robustness2::NAME.as_ptr(),
        ];
        if ray_query {
            names.extend([
                ash::khr::ray_query::NAME.as_ptr(),
                ash::khr::acceleration_structure::NAME.as_ptr(),
                ash::khr::deferred_host_operations::NAME.as_ptr(),
            ]);
        }
        let core = vk::PhysicalDeviceFeatures::default().robust_buffer_access(true);
        let mut vulkan12 = vk::PhysicalDeviceVulkan12Features::default()
            .buffer_device_address(ray_query)
            .vulkan_memory_model(true);
        let mut executable_info =
            vk::PhysicalDevicePipelineExecutablePropertiesFeaturesKHR::default()
                .pipeline_executable_info(true);
        let mut robustness = vk::PhysicalDeviceRobustness2FeaturesEXT::default()
            .robust_buffer_access2(true)
            .robust_image_access2(true);
        let mut ray_query_features =
            vk::PhysicalDeviceRayQueryFeaturesKHR::default().ray_query(true);
        let mut acceleration_features =
            vk::PhysicalDeviceAccelerationStructureFeaturesKHR::default()
                .acceleration_structure(true);
        let mut create = vk::DeviceCreateInfo::default()
            .queue_create_infos(&queues)
            .enabled_extension_names(&names)
            .enabled_features(&core)
            .push_next(&mut vulkan12)
            .push_next(&mut executable_info)
            .push_next(&mut robustness);
        if ray_query {
            create = create
                .push_next(&mut ray_query_features)
                .push_next(&mut acceleration_features);
        }
        // SAFETY: every requested extension was checked above, and the feature chain is valid.
        let device = unsafe { instance.create_device(physical, &create, None) }
            .expect("create Vulkan device");
        let executables = ash::khr::pipeline_executable_properties::Device::new(&instance, &device);
        Some(Self {
            _entry: entry,
            instance,
            device,
            executables,
            adapter,
            ray_query,
        })
    }

    /// Compiles one compute entry point with statistics capture and returns the driver's
    /// `(name, value)` pairs for each executable it produced.
    fn statistics(
        &self,
        words: &[u32],
        module: &naga::Module,
        entry: &str,
    ) -> Vec<(String, String)> {
        let device = &self.device;
        // SAFETY: valid SPIR-V from naga, and every handle created here is destroyed below.
        unsafe {
            let shader = device
                .create_shader_module(&vk::ShaderModuleCreateInfo::default().code(words), None)
                .expect("shader module");

            let all = bindings(module);
            let groups = all.iter().map(|(g, ..)| g + 1).max().unwrap_or(0);
            let layouts: Vec<vk::DescriptorSetLayout> = (0..groups)
                .map(|group| {
                    let entries: Vec<_> = all
                        .iter()
                        .filter(|(g, ..)| *g == group)
                        .map(|(_, binding, kind)| {
                            vk::DescriptorSetLayoutBinding::default()
                                .binding(*binding)
                                .descriptor_type(*kind)
                                .descriptor_count(1)
                                .stage_flags(vk::ShaderStageFlags::COMPUTE)
                        })
                        .collect();
                    device
                        .create_descriptor_set_layout(
                            &vk::DescriptorSetLayoutCreateInfo::default().bindings(&entries),
                            None,
                        )
                        .expect("descriptor set layout")
                })
                .collect();
            let layout = device
                .create_pipeline_layout(
                    &vk::PipelineLayoutCreateInfo::default().set_layouts(&layouts),
                    None,
                )
                .expect("pipeline layout");

            let name = CString::new(entry).unwrap();
            let stage = vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::COMPUTE)
                .module(shader)
                .name(&name);
            let info = vk::ComputePipelineCreateInfo::default()
                .stage(stage)
                .layout(layout)
                .flags(vk::PipelineCreateFlags::CAPTURE_STATISTICS_KHR);
            let pipeline = device
                .create_compute_pipelines(vk::PipelineCache::null(), &[info], None)
                .map_err(|(_, e)| e)
                .expect("compute pipeline")[0];

            let mut out = Vec::new();
            let executables = self
                .executables
                .get_pipeline_executable_properties(
                    &vk::PipelineInfoKHR::default().pipeline(pipeline),
                )
                .expect("executable properties");
            for index in 0..executables.len() as u32 {
                let stats = self
                    .executables
                    .get_pipeline_executable_statistics(
                        &vk::PipelineExecutableInfoKHR::default()
                            .pipeline(pipeline)
                            .executable_index(index),
                    )
                    .expect("executable statistics");
                for stat in stats {
                    let value = match stat.format {
                        vk::PipelineExecutableStatisticFormatKHR::BOOL32 => {
                            (stat.value.b32 != 0).to_string()
                        }
                        vk::PipelineExecutableStatisticFormatKHR::INT64 => {
                            stat.value.i64.to_string()
                        }
                        vk::PipelineExecutableStatisticFormatKHR::UINT64 => {
                            stat.value.u64.to_string()
                        }
                        _ => format!("{:.3}", stat.value.f64),
                    };
                    let name = stat
                        .name_as_c_str()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    out.push((name, value));
                }
            }

            device.destroy_pipeline(pipeline, None);
            device.destroy_pipeline_layout(layout, None);
            for set in layouts {
                device.destroy_descriptor_set_layout(set, None);
            }
            device.destroy_shader_module(shader, None);
            out
        }
    }
}

impl Drop for Driver {
    fn drop(&mut self) {
        // SAFETY: nothing created from these handles outlives them.
        unsafe {
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}

#[test]
#[ignore]
fn shader_statistics() {
    let Some(driver) = Driver::new() else { return };
    eprintln!("Driver statistics on {} (Vulkan)", driver.adapter);
    for (label, source, entry) in KERNELS {
        if label.contains("ray query") && !driver.ray_query {
            eprintln!("  {label}: skipped, no ray queries");
            continue;
        }
        let module = naga::front::wgsl::parse_str(source).unwrap();
        let words = spirv(label, source);
        let stats = driver.statistics(&words, &module, entry);
        let line: Vec<String> = stats.iter().map(|(n, v)| format!("{n} {v}")).collect();
        eprintln!("  {label}: {}", line.join(", "));
    }
}
