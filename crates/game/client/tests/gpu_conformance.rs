//! Holds the production WGSL to the CPU reference.
//!
//! - Every voxel shader must parse and validate with naga, no GPU needed.
//! - On a real adapter, the WGSL mask prefix count must match the CPU bit-for-bit for every child
//!   index of every test mask, and the WGSL traversal of both local hierarchies (64-tree and
//!   octree) must satisfy the same acceptance rules as the CPU traversal (`voxel::reference`) on
//!   boundary torture rays, for both the single-page fast path and the multi-page page-table path.
//! - Whole-scene GPU traversal must find the CPU's nearest hits for every scene organization and
//!   hierarchy.
//! - The 64-tree traversal with footprint LOD filtering must take exactly the CPU reference's path
//!   on the real assets, in both residency layouts, and whole-scene filtered traversal must find
//!   the CPU's nearest filtered hits for every organization, including the hardware broad phase.
//!
//! GPU checks skip (and say so) when no adapter is available, e.g. on headless CI runners.

use bevy::{
    math::{UVec3, Vec3},
    tasks::block_on,
};
use bytemuck::{Pod, Zeroable};
use client::render::gpu::layout::{GpuGeometryMeta, root_bounds, root_flags};
use voxel::{
    ArenaHandle, GeometryArenas, GeometrySource, HierarchyKind, Mask64, Ray, VoxelModule,
    VoxelRoot,
    hierarchy::ATTRIBUTE_MATERIAL_MASK,
    reference::check_hit,
    residency::{ResidencyLimits, plan_slabs},
};
use wesl_core::include_wesl;
use wgpu::util::DeviceExt;

const PRIMARY_VISIBILITY: &str = include_wesl!("primary_visibility");
const DIRECT_LIGHTING: &str = include_wesl!("direct_lighting");
const CONFORMANCE: &str = include_wesl!("voxel_conformance");
const PRIMARY_VISIBILITY_MULTI_SLAB: &str = include_wesl!("primary_visibility_multi_slab");
const DIRECT_LIGHTING_MULTI_SLAB: &str = include_wesl!("direct_lighting_multi_slab");
const CONFORMANCE_MULTI_SLAB: &str = include_wesl!("voxel_conformance_multi_slab");
const CONFORMANCE_STACKLESS: &str = include_wesl!("voxel_conformance_stackless");
const PRIMARY_VISIBILITY_RAY_QUERY: &str = include_wesl!("primary_visibility_ray_query");
const DIRECT_LIGHTING_RAY_QUERY: &str = include_wesl!("direct_lighting_ray_query");
const CONFORMANCE_RAY_QUERY: &str = include_wesl!("voxel_conformance_ray_query");
const PRIMARY_VISIBILITY_FOOTPRINT_LOD: &str = include_wesl!("primary_visibility_footprint_lod");
const PRIMARY_VISIBILITY_FOOTPRINT_LOD_MULTI_SLAB: &str =
    include_wesl!("primary_visibility_footprint_lod_multi_slab");
const PRIMARY_VISIBILITY_FOOTPRINT_LOD_RAY_QUERY: &str =
    include_wesl!("primary_visibility_footprint_lod_ray_query");
const CONFORMANCE_FOOTPRINT_LOD: &str = include_wesl!("voxel_conformance_footprint_lod");
const CONFORMANCE_FOOTPRINT_LOD_MULTI_SLAB: &str =
    include_wesl!("voxel_conformance_footprint_lod_multi_slab");
const CONFORMANCE_FOOTPRINT_LOD_RAY_QUERY: &str =
    include_wesl!("voxel_conformance_footprint_lod_ray_query");

/// How long any one submission may take before the test fails instead of waiting forever. wgpu's
/// Metal backend only stops waiting once a command buffer completes, so a faulted one would
/// otherwise hang the test.
const GPU_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

// INFO: -----------------------------
//         shader validation
// -----------------------------------

#[test]
fn shaders_validate() {
    for (name, source) in [
        ("primary_visibility", PRIMARY_VISIBILITY),
        ("direct_lighting", DIRECT_LIGHTING),
        ("voxel_conformance", CONFORMANCE),
        (
            "primary_visibility_multi_slab",
            PRIMARY_VISIBILITY_MULTI_SLAB,
        ),
        ("direct_lighting_multi_slab", DIRECT_LIGHTING_MULTI_SLAB),
        ("voxel_conformance_multi_slab", CONFORMANCE_MULTI_SLAB),
        ("voxel_conformance_stackless", CONFORMANCE_STACKLESS),
        ("primary_visibility_ray_query", PRIMARY_VISIBILITY_RAY_QUERY),
        ("direct_lighting_ray_query", DIRECT_LIGHTING_RAY_QUERY),
        ("voxel_conformance_ray_query", CONFORMANCE_RAY_QUERY),
        (
            "primary_visibility_footprint_lod",
            PRIMARY_VISIBILITY_FOOTPRINT_LOD,
        ),
        (
            "primary_visibility_footprint_lod_multi_slab",
            PRIMARY_VISIBILITY_FOOTPRINT_LOD_MULTI_SLAB,
        ),
        (
            "primary_visibility_footprint_lod_ray_query",
            PRIMARY_VISIBILITY_FOOTPRINT_LOD_RAY_QUERY,
        ),
        ("voxel_conformance_footprint_lod", CONFORMANCE_FOOTPRINT_LOD),
        (
            "voxel_conformance_footprint_lod_multi_slab",
            CONFORMANCE_FOOTPRINT_LOD_MULTI_SLAB,
        ),
        (
            "voxel_conformance_footprint_lod_ray_query",
            CONFORMANCE_FOOTPRINT_LOD_RAY_QUERY,
        ),
    ] {
        let module = naga::front::wgsl::parse_str(source)
            .unwrap_or_else(|e| panic!("{name} failed to parse:\n{}", e.emit_to_string(source)));
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            // the hardware broad phase variants need ray queries, and LOD summaries unpack f16
            // pairs into f32 (`unpack2x16float`, which every native backend has)
            naga::valid::Capabilities::default()
                | naga::valid::Capabilities::RAY_QUERY
                | naga::valid::Capabilities::SHADER_FLOAT16_IN_FLOAT32,
        )
        .validate(&module)
        .unwrap_or_else(|e| panic!("{name} failed validation:\n{}", e.emit_to_string(source)));
    }
}

// INFO: ---------------------
//         gpu harness
// ---------------------------

/// Which build of the conformance kernels to run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Variant {
    /// The production single-slab kernels.
    Default,
    /// For geometry spread over several slabs.
    MultiSlab,
    /// The stackless 64-tree traversal-state experiment (single slab).
    Stackless,
    /// The hardware scene broad phase: ray queries over flattened pose groups (single slab).
    RayQuery,
    /// 64-tree footprint LOD filtering (single slab).
    FootprintLod,
    /// 64-tree footprint LOD filtering for geometry spread over several slabs.
    FootprintLodMultiSlab,
    /// 64-tree footprint LOD filtering with the hardware scene broad phase (single slab).
    FootprintLodRayQuery,
}

impl Variant {
    /// The production variant for geometry in `slabs` topology slabs.
    fn for_slabs(slabs: usize) -> Self {
        if slabs > 1 {
            Self::MultiSlab
        } else {
            Self::Default
        }
    }
}

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    /// Conformance kernels, indexed by [`Variant`]; `None` where the device can't run them.
    modules: [Option<wgpu::ShaderModule>; 7],
    /// Whether the device has hardware ray queries (and so a [`Variant::RayQuery`] kernel).
    ray_query: bool,
    /// Whether [`Gpu::gpu_ms`] can time dispatches with timestamp queries.
    timestamps: bool,
}

/// Bindings for one bind group: `(group, [(binding, resource)])`.
type Group<'a> = (u32, Vec<(u32, wgpu::BindingResource<'a>)>);

/// The adapter wgpu prefers (Vulkan before DX12 on Windows), or the one `WGPU_BACKEND` (e.g.
/// `vulkan`, `dx12`, `metal`) and `WGPU_ADAPTER_NAME` select, as the game itself honors them.
fn gpu() -> Option<Gpu> {
    let instance =
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let adapter = match block_on(wgpu::util::initialize_adapter_from_env_or_default(
        &instance, None,
    )) {
        Ok(adapter) => adapter,
        Err(e) => {
            eprintln!("skipping GPU conformance: no adapter ({e})");
            return None;
        }
    };
    let info = adapter.get_info();
    eprintln!("GPU conformance on {} ({:?})", info.name, info.backend);
    let ray_query = client::render::device::ray_query_supported(info.backend, adapter.features());
    let timestamps = adapter.features().contains(wgpu::Features::TIMESTAMP_QUERY);
    let skipped = !ray_query
        && adapter
            .features()
            .contains(wgpu::Features::EXPERIMENTAL_RAY_QUERY);
    eprintln!(
        "  hardware ray queries: {ray_query}{}, timestamp queries: {timestamps}",
        if skipped {
            " (adapter has them, unsupported on this backend)"
        } else {
            ""
        }
    );
    let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("conformance"),
        required_features: adapter.features()
            & (wgpu::Features::TIMESTAMP_QUERY
                | if ray_query {
                    wgpu::Features::EXPERIMENTAL_RAY_QUERY
                } else {
                    wgpu::Features::empty()
                }),
        required_limits: adapter.limits(),
        // SAFETY: ray queries are the only experimental feature requested, and they are only
        // used by the broad phase kernels, which the tests hold to the CPU reference.
        experimental_features: if ray_query {
            unsafe { wgpu::ExperimentalFeatures::enabled() }
        } else {
            wgpu::ExperimentalFeatures::disabled()
        },
        ..Default::default()
    }))
    .expect("device");
    let modules = [
        ("voxel_conformance", CONFORMANCE, true),
        ("voxel_conformance_multi_slab", CONFORMANCE_MULTI_SLAB, true),
        ("voxel_conformance_stackless", CONFORMANCE_STACKLESS, true),
        (
            "voxel_conformance_ray_query",
            CONFORMANCE_RAY_QUERY,
            ray_query,
        ),
        (
            "voxel_conformance_footprint_lod",
            CONFORMANCE_FOOTPRINT_LOD,
            true,
        ),
        (
            "voxel_conformance_footprint_lod_multi_slab",
            CONFORMANCE_FOOTPRINT_LOD_MULTI_SLAB,
            true,
        ),
        (
            "voxel_conformance_footprint_lod_ray_query",
            CONFORMANCE_FOOTPRINT_LOD_RAY_QUERY,
            ray_query,
        ),
    ]
    .map(|(label, source, supported)| {
        supported.then(|| {
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            })
        })
    });
    Some(Gpu {
        device,
        queue,
        modules,
        ray_query,
        timestamps,
    })
}

impl Gpu {
    fn storage<T: Pod>(&self, data: &[T]) -> wgpu::Buffer {
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(data),
                usage: wgpu::BufferUsages::STORAGE,
            })
    }

    fn uniform<T: Pod>(&self, data: &T) -> wgpu::Buffer {
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::bytes_of(data),
                usage: wgpu::BufferUsages::UNIFORM,
            })
    }

    fn output(&self, bytes: u64) -> wgpu::Buffer {
        self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        })
    }

    /// Runs `entry` over `count` items, then reads back `output`. The pipeline uses an automatic
    /// layout, which only contains the bindings the entry point actually uses.
    fn run<R: Pod>(
        &self,
        entry: &str,
        count: u32,
        groups: &[Group<'_>],
        output: &wgpu::Buffer,
    ) -> Vec<R> {
        self.run_variant(entry, Variant::Default, count, groups, output)
    }

    /// [`run`](Self::run) with a chosen build of the kernels.
    fn run_variant<R: Pod>(
        &self,
        entry: &str,
        variant: Variant,
        count: u32,
        groups: &[Group<'_>],
        output: &wgpu::Buffer,
    ) -> Vec<R> {
        let (pipeline, bind_groups) = self.bind(entry, variant, groups);

        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: output.size(),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pipeline);
            for (index, group) in &bind_groups {
                pass.set_bind_group(*index, group, &[]);
            }
            pass.dispatch_workgroups(count.div_ceil(64), 1, 1);
        }
        encoder.copy_buffer_to_buffer(output, 0, &readback, 0, output.size());
        self.queue.submit([encoder.finish()]);
        self.read(&readback)
    }

    /// GPU time of one dispatch of `entry` over `count` items, in milliseconds: the mean of
    /// `runs` dispatches measured with timestamp queries around each compute pass, so neither
    /// readback nor pipeline creation is included. `None` without timestamp queries.
    fn gpu_ms(
        &self,
        entry: &str,
        variant: Variant,
        count: u32,
        groups: &[Group<'_>],
        runs: u32,
    ) -> Option<f64> {
        if !self.timestamps {
            return None;
        }
        let (pipeline, bind_groups) = self.bind(entry, variant, groups);
        self.time_dispatch(&pipeline, &bind_groups, [count.div_ceil(64), 1, 1], runs)
    }

    /// GPU time of one `workgroups` dispatch of `pipeline`, in milliseconds, as
    /// [`gpu_ms`](Self::gpu_ms) measures it. `None` without timestamp queries.
    fn time_dispatch(
        &self,
        pipeline: &wgpu::ComputePipeline,
        bind_groups: &[(u32, wgpu::BindGroup)],
        [x, y, z]: [u32; 3],
        runs: u32,
    ) -> Option<f64> {
        if !self.timestamps {
            return None;
        }
        let queries = self.device.create_query_set(&wgpu::QuerySetDescriptor {
            label: None,
            ty: wgpu::QueryType::Timestamp,
            count: runs * 2,
        });
        let resolve = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: runs as u64 * 16,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: runs as u64 * 16,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        for run in 0..runs {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: None,
                timestamp_writes: Some(wgpu::ComputePassTimestampWrites {
                    query_set: &queries,
                    beginning_of_pass_write_index: Some(run * 2),
                    end_of_pass_write_index: Some(run * 2 + 1),
                }),
            });
            pass.set_pipeline(pipeline);
            for (index, group) in bind_groups {
                pass.set_bind_group(*index, group, &[]);
            }
            pass.dispatch_workgroups(x, y, z);
        }
        encoder.resolve_query_set(&queries, 0..runs * 2, &resolve, 0);
        encoder.copy_buffer_to_buffer(&resolve, 0, &readback, 0, resolve.size());
        self.queue.submit([encoder.finish()]);
        let ticks: Vec<u64> = self.read(&readback);
        let period_ns = self.queue.get_timestamp_period() as f64;
        let total: u64 = ticks.chunks(2).map(|t| t[1].saturating_sub(t[0])).sum();
        Some(total as f64 * period_ns / 1e6 / runs as f64)
    }

    /// Waits for every submission so far, failing after [`GPU_TIMEOUT`].
    fn wait(&self) {
        if let Err(e) = self.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(GPU_TIMEOUT),
        }) {
            panic!("GPU work did not complete ({e}); the command buffer likely faulted");
        }
    }

    /// Waits for the queue and copies a mappable buffer's contents out.
    fn read<R: Pod>(&self, readback: &wgpu::Buffer) -> Vec<R> {
        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        self.wait();
        bytemuck::cast_slice(&slice.get_mapped_range()).to_vec()
    }

    /// A pipeline for `entry` with an automatic layout (only the bindings it reads), and its
    /// bind groups.
    fn bind(
        &self,
        entry: &str,
        variant: Variant,
        groups: &[Group<'_>],
    ) -> (wgpu::ComputePipeline, Vec<(u32, wgpu::BindGroup)>) {
        let module = self.modules[variant as usize]
            .as_ref()
            .expect("the device doesn't support this kernel variant");
        self.bind_module(module, entry, groups)
    }

    /// [`bind`](Self::bind) for any shader module, such as a production pass.
    fn bind_module(
        &self,
        module: &wgpu::ShaderModule,
        entry: &str,
        groups: &[Group<'_>],
    ) -> (wgpu::ComputePipeline, Vec<(u32, wgpu::BindGroup)>) {
        let pipeline = self
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: None,
                module,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            });
        let bind_groups: Vec<(u32, wgpu::BindGroup)> = groups
            .iter()
            .map(|(index, bindings)| {
                let entries: Vec<wgpu::BindGroupEntry> = bindings
                    .iter()
                    .map(|(binding, resource)| wgpu::BindGroupEntry {
                        binding: *binding,
                        resource: resource.clone(),
                    })
                    .collect();
                let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &pipeline.get_bind_group_layout(*index),
                    entries: &entries,
                });
                (*index, group)
            })
            .collect();
        (pipeline, bind_groups)
    }
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: u32) -> u32 {
        (self.next() % n as u64) as u32
    }

    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * ((self.next() >> 40) as f32 / (1u64 << 24) as f32)
    }

    fn direction(&mut self) -> Vec3 {
        loop {
            let v = Vec3::new(
                self.range(-1.0, 1.0),
                self.range(-1.0, 1.0),
                self.range(-1.0, 1.0),
            );
            if v.length_squared() > 0.01 && v.length_squared() <= 1.0 {
                return v.normalize();
            }
        }
    }
}

// INFO: ---------------------
//         mask kernel
// ---------------------------

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct MaskCase {
    mask: [u32; 2],
    k: u32,
    pad: u32,
}

#[test]
fn gpu_mask_prefix_counts_match_cpu() {
    let Some(gpu) = gpu() else { return };

    let mut masks = vec![
        0u64,
        u64::MAX,
        1 << 31,
        1 << 32,
        (1 << 31) | (1 << 32),
        0xFFFF_FFFF,
        !0xFFFF_FFFF,
    ];
    masks.extend((0..64).flat_map(|b| [1u64 << b, !(1u64 << b)]));
    let mut rng = Rng(0x5EED);
    masks.extend((0..512).map(|_| rng.next()));
    masks.extend((0..512).map(|_| rng.next() & rng.next() & rng.next()));

    let cases: Vec<MaskCase> = masks
        .iter()
        .flat_map(|&bits| {
            let m = Mask64::from_u64(bits);
            (0..64).map(move |k| MaskCase {
                mask: [m.lo, m.hi],
                k,
                pad: 0,
            })
        })
        .collect();

    let input = gpu.storage(&cases);
    let output = gpu.output(cases.len() as u64 * 8);
    let results: Vec<[u32; 2]> = gpu.run(
        "masks",
        cases.len() as u32,
        // groups 0 and 1 are empty for this entry point, but the automatic layout still expects
        // them bound
        &[
            (0, vec![]),
            (1, vec![]),
            (
                2,
                vec![
                    (0, input.as_entire_binding()),
                    (1, output.as_entire_binding()),
                ],
            ),
        ],
        &output,
    );

    for (case, result) in cases.iter().zip(&results) {
        let mask = Mask64 {
            lo: case.mask[0],
            hi: case.mask[1],
        };
        assert_eq!(
            *result,
            [mask.prefix_count(case.k), mask.child_word_offset(case.k)],
            "mask {:#018x} child {}",
            mask.to_u64(),
            case.k
        );
    }
}

// INFO: ---------------------
//         ray kernel
// ---------------------------

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct RayCase {
    origin: [f32; 3],
    t_min: f32,
    dir: [f32; 3],
    t_max: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct RayResult {
    hit: u32,
    t: f32,
    cell: u32,
    attribute: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct RootParams {
    node: u32,
    levels: u32,
    page_base: u32,
    flags: u32,
    bounds_min: [u32; 3],
    pad0: u32,
    bounds_max: [u32; 3],
    pad1: u32,
}

impl RootParams {
    fn new(root: &VoxelRoot, page_base: u32) -> Self {
        let (bounds_min, bounds_max) = root_bounds(root);
        Self {
            node: root.node().raw(),
            levels: root.levels() as u32,
            page_base,
            flags: root_flags(root),
            bounds_min,
            pad0: 0,
            bounds_max,
            pad1: 0,
        }
    }
}

/// Voxels clustered on both sides of every node boundary of a 1024^3 module, plus noise.
fn boundary_module(rng: &mut Rng) -> VoxelModule {
    let mut module = VoxelModule::new(5);
    let coords = [0, 3, 4, 15, 16, 63, 64, 255, 256, 511, 512, 1023];
    for &x in &coords {
        for &y in &coords {
            for &z in &coords {
                if rng.below(3) == 0 {
                    module.set(UVec3::new(x, y, z), 1 + rng.below(9) as u16);
                }
            }
        }
    }
    for _ in 0..600 {
        let p = UVec3::new(rng.below(1024), rng.below(1024), rng.below(1024));
        module.set(p, 1 + rng.below(9) as u16);
    }
    module
}

/// Rays aimed at voxel corners, edges, and faces: diagonal, axis-aligned, nearly parallel to a
/// plane, and random.
fn torture_rays(module: &VoxelModule, rng: &mut Rng, count: usize) -> Vec<Ray> {
    // sorted, since module iteration order is not deterministic
    let mut voxels: Vec<UVec3> = module.voxels().map(|(p, _)| p).collect();
    voxels.sort_unstable_by_key(|p| (p.z, p.y, p.x));
    let voxels: Vec<Vec3> = voxels.into_iter().map(|p| p.as_vec3()).collect();
    (0..count)
        .map(|_| {
            let mut offset = || [0.0, 0.5, 1.0][rng.below(3) as usize];
            let offset = Vec3::new(offset(), offset(), offset());
            let target = voxels[rng.below(voxels.len() as u32) as usize] + offset;
            let mut sign = || if rng.below(2) == 0 { 1.0 } else { -1.0 };
            let diagonal = Vec3::new(sign(), sign(), sign()).normalize();
            let dir = match rng.below(4) {
                0 => diagonal,
                1 => [
                    Vec3::X,
                    Vec3::NEG_X,
                    Vec3::Y,
                    Vec3::NEG_Y,
                    Vec3::Z,
                    Vec3::NEG_Z,
                ][rng.below(6) as usize],
                2 => {
                    let mut d = rng.direction();
                    d[rng.below(3) as usize] *= 1e-4;
                    d.normalize()
                }
                _ => rng.direction(),
            };
            Ray::new(target - dir * rng.range(2.0, 1500.0), dir)
        })
        .collect()
}

/// The topology slab bindings of the geometry group, repeating slab 0 for unused slabs exactly as
/// the renderer does. Only the multi-slab variant reads slabs 1 to 3, and the automatic pipeline
/// layout omits bindings a shader never reads.
fn topology_bindings(slabs: &[wgpu::Buffer]) -> Vec<(u32, wgpu::BindingResource<'_>)> {
    let bindings: &[u32] = if slabs.len() > 1 { &[0, 5, 6, 7] } else { &[0] };
    bindings
        .iter()
        .copied()
        .enumerate()
        .map(|(slab, binding)| {
            (
                binding,
                slabs.get(slab).unwrap_or(&slabs[0]).as_entire_binding(),
            )
        })
        .collect()
}

fn check_gpu_traversal(
    gpu: &Gpu,
    module: &VoxelModule,
    arenas: &GeometryArenas,
    root: &VoxelRoot,
    rays: &[Ray],
    limits: &ResidencyLimits,
    stackless: bool,
) -> Vec<RayResult> {
    // pack exactly as the client does
    let plan = plan_slabs(
        "topology",
        arenas.topology.pages().iter().map(Vec::len),
        limits,
    )
    .unwrap();
    let slabs: Vec<wgpu::Buffer> = plan
        .pack(arenas.topology.pages())
        .iter()
        .map(|slab| gpu.storage(slab))
        .collect();
    let params = RootParams::new(root, plan.page_entries[root.node().page() as usize]);
    let cases: Vec<RayCase> = rays
        .iter()
        .map(|r| RayCase {
            origin: r.origin.to_array(),
            t_min: 0.0,
            dir: r.dir.to_array(),
            t_max: f32::MAX,
        })
        .collect();

    let meta_buffer = gpu.uniform(&GpuGeometryMeta::new(&plan.page_entries, &[]));
    let ray_buffer = gpu.storage(&cases);
    let params_buffer = gpu.uniform(&params);
    let output = gpu.output(cases.len() as u64 * 16);
    let variant = if stackless {
        Variant::Stackless
    } else {
        Variant::for_slabs(slabs.len())
    };
    let results: Vec<RayResult> = gpu.run_variant(
        "rays",
        variant,
        cases.len() as u32,
        &[
            (0, {
                let mut bindings = topology_bindings(&slabs);
                bindings.push((4, meta_buffer.as_entire_binding()));
                bindings
            }),
            (1, vec![]),
            (
                2,
                vec![
                    (2, ray_buffer.as_entire_binding()),
                    (3, output.as_entire_binding()),
                    (4, params_buffer.as_entire_binding()),
                ],
            ),
        ],
        &output,
    );

    let mut failures = Vec::new();
    let mut identical = 0;
    for (ray, result) in rays.iter().zip(&results) {
        let cell = UVec3::new(
            result.cell & 0x3FF,
            (result.cell >> 10) & 0x3FF,
            result.cell >> 20,
        );
        let gpu_hit = (result.hit != 0).then_some((result.t, cell));
        if let Err(e) = check_hit(module, ray, 0.0, f32::MAX, gpu_hit) {
            failures.push(format!("{e}\n  ray {ray:?}"));
        } else if gpu_hit.is_some() {
            // the attribute handle must name the reported cell's own material
            let word = arenas.attribute_word(ArenaHandle::from_raw(result.attribute));
            if (word & ATTRIBUTE_MATERIAL_MASK) as u16 != module.get(cell) {
                failures.push(format!(
                    "attribute handle of cell {cell} names the wrong voxel"
                ));
            }
        }
        let cpu = root.trace(arenas, ray, 0.0, f32::MAX).map(|h| h.cell);
        identical += (cpu == gpu_hit.map(|h| h.1)) as usize;
    }
    eprintln!(
        "  {:?} ({variant:?}), single page {}, {} slab(s): {} rays pass, {} pick the same cell as the CPU traversal",
        root.kind(),
        root.single_page(),
        plan.slab_count(),
        rays.len() - failures.len(),
        identical
    );
    assert!(
        failures.is_empty(),
        "{} GPU traversal failures, first:\n{}",
        failures.len(),
        failures[0]
    );
    results
}

#[test]
fn gpu_hierarchy_traversal_matches_reference() {
    let Some(gpu) = gpu() else { return };
    let mut rng = Rng(42);
    let module = boundary_module(&mut rng);
    let rays = torture_rays(&module, &mut rng, 20_000);

    let unlimited = ResidencyLimits {
        max_storage_buffer_binding_size: 1 << 30,
        max_buffer_size: 1 << 30,
    };
    for kind in HierarchyKind::ALL {
        // the single-page fast path, then the page-table path with tiny pages
        for page_words in [voxel::handle::MAX_PAGE_WORDS, 256] {
            let mut arenas = GeometryArenas::with_page_words(page_words);
            let root = arenas.bake(&module, kind).unwrap();
            assert_eq!(root.single_page(), page_words != 256);
            let stack =
                check_gpu_traversal(&gpu, &module, &arenas, &root, &rays, &unlimited, false);

            if kind == HierarchyKind::Tree64 {
                // the stackless traversal-state experiment must be bit-identical, not merely
                // acceptable, since it only changes where parent state comes from
                let stackless =
                    check_gpu_traversal(&gpu, &module, &arenas, &root, &rays, &unlimited, true);
                let same = stack
                    .iter()
                    .zip(&stackless)
                    .filter(|(a, b)| bytemuck::bytes_of(*a) == bytemuck::bytes_of(*b))
                    .count();
                assert_eq!(
                    same,
                    rays.len(),
                    "stackless results differ from the stack's"
                );
            }

            if page_words == 256 {
                // and spread across slabs: every node fetch crosses the slab switch, and child
                // blocks land in different bindings than their parents
                let bytes = arenas.topology.used_words() * 4;
                let limits = ResidencyLimits {
                    max_storage_buffer_binding_size: (bytes / 3).max(256 * 4),
                    max_buffer_size: 1 << 30,
                };
                check_gpu_traversal(&gpu, &module, &arenas, &root, &rays, &limits, false);
            }
        }
    }
}

// INFO: ---------------------------------
//         footprint filtering kernel
// ---------------------------------------

/// `voxel::lod::FootprintFilter`, padded for the uniform.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct FilterParams {
    radius: f32,
    spread: f32,
    threshold: f32,
    pad: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct FilteredRayResult {
    hit: u32,
    t: f32,
    cell: u32,
    lod: u32,
    material: u32,
    steps: u32,
    /// The hit frame, for whole-scene queries.
    frame: u32,
    /// A filtered hit's LOD record index and the summary normal the shader decoded from it
    /// (bare 64-tree queries only).
    record: u32,
    normal: [f32; 3],
    pad: u32,
}

/// Slab bindings of one geometry arena at `bindings` (slab 0 first), repeating slab 0 for unused
/// slabs; single-slab kernels only read slab 0.
fn slab_bindings(
    slabs: &[wgpu::Buffer],
    bindings: [u32; 4],
) -> Vec<(u32, wgpu::BindingResource<'_>)> {
    let used = if slabs.len() > 1 { 4 } else { 1 };
    bindings[..used]
        .iter()
        .enumerate()
        .map(|(slab, &binding)| {
            (
                binding,
                slabs.get(slab).unwrap_or(&slabs[0]).as_entire_binding(),
            )
        })
        .collect()
}

/// The WGSL 64-tree traversal with footprint LOD filtering must take the CPU reference's path
/// exactly (`tree64::trace_filtered`): the same hit or miss, cell, level, material, and DDA step
/// count, on the real assets, for exact, narrow, and wide cones, in both residency layouts.
#[test]
fn gpu_footprint_filtering_matches_cpu() {
    use client::render::geometry::GpuGeometryLibrary;
    use voxel::{
        hierarchy::tree64,
        lod::{FootprintFilter, LodSummary},
    };

    let Some(gpu) = gpu() else { return };
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let library = common::world::assets::load_library(
        &workspace.join("assets/shared/vanta"),
        &std::env::temp_dir().join("vanta-conformance-cache"),
    );
    let unlimited = ResidencyLimits {
        max_storage_buffer_binding_size: 1 << 30,
        max_buffer_size: 1 << 30,
    };
    let single = GpuGeometryLibrary::build(&library, &unlimited).unwrap();
    let page_words = 1 << 16;
    let slab_bytes = (single.stats.topology_bytes / 2)
        .max(single.stats.attribute_bytes.div_ceil(4))
        + page_words as u64 * 4;
    let limits = ResidencyLimits {
        max_storage_buffer_binding_size: slab_bytes,
        max_buffer_size: 1 << 30,
    };
    let multi = GpuGeometryLibrary::build_with_page_words(&library, &limits, page_words).unwrap();
    assert!(
        multi.topology.len() > 1,
        "the multi-slab layout should split topology"
    );

    let filters = [
        // exact: no cell fits a zero cone
        FootprintFilter {
            radius: 0.0,
            spread: 0.0,
            threshold: 0.5,
        },
        // a pixel-like cone: bricks filter only far away
        FootprintFilter {
            radius: 0.25,
            spread: 0.01,
            threshold: 0.5,
        },
        // wide, with a threshold low enough that sparse foliage stays visible (a radius of exactly
        // 2 would make a brick exactly fit a unit ray at t = 0, a tie `length` can round either way)
        FootprintFilter {
            radius: 2.1,
            spread: 0.05,
            threshold: 0.1,
        },
        // a mid-range, stochastic-style threshold
        FootprintFilter {
            radius: 1.0,
            spread: 0.02,
            threshold: 0.37,
        },
    ];

    for geometry in [single, multi] {
        let variant = if geometry.multi_slab() {
            Variant::FootprintLodMultiSlab
        } else {
            Variant::FootprintLod
        };
        let topology: Vec<wgpu::Buffer> =
            geometry.topology.iter().map(|s| gpu.storage(s)).collect();
        let attributes: Vec<wgpu::Buffer> =
            geometry.attributes.iter().map(|s| gpu.storage(s)).collect();
        let lod = gpu.storage(&geometry.lod);
        let meta = gpu.uniform(&GpuGeometryMeta::new(
            &geometry.topology_page_entries,
            &geometry.attribute_page_entries,
        ));

        let mut rng = Rng(23);
        let mut rays = 0;
        let mut filtered = [0usize; 4];
        let mut failures = Vec::new();
        for (asset, asset_geometry) in library.assets().iter().zip(&geometry.assets) {
            for module in 0..asset.assembly.modules.len() {
                let root = asset_geometry.root(module as u16, HierarchyKind::Tree64);
                let VoxelRoot::Tree64(tree) = root else {
                    unreachable!("the 64-tree slot holds a 64-tree root")
                };
                if tree.bounds.is_empty() {
                    continue;
                }
                let extent = tree.extent() as f32;
                let module_rays: Vec<Ray> = (0..1024)
                    .map(|_| {
                        let target = tree.bounds.min
                            + (tree.bounds.max - tree.bounds.min)
                                * Vec3::new(
                                    rng.range(0.0, 1.0),
                                    rng.range(0.0, 1.0),
                                    rng.range(0.0, 1.0),
                                );
                        let dir = match rng.below(3) {
                            0 => [
                                Vec3::X,
                                Vec3::NEG_X,
                                Vec3::Y,
                                Vec3::NEG_Y,
                                Vec3::Z,
                                Vec3::NEG_Z,
                            ][rng.below(6) as usize],
                            _ => rng.direction(),
                        };
                        // pose frames scale directions into module voxels
                        let scale = [1.0, 0.5, 3.0][rng.below(3) as usize];
                        Ray::new(target - dir * rng.range(1.0, extent * 4.0), dir * scale)
                    })
                    .collect();
                rays += module_rays.len();
                let cases: Vec<RayCase> = module_rays
                    .iter()
                    .map(|r| RayCase {
                        origin: r.origin.to_array(),
                        t_min: 0.0,
                        dir: r.dir.to_array(),
                        t_max: f32::MAX,
                    })
                    .collect();
                let ray_buffer = gpu.storage(&cases);
                let params = gpu.uniform(&RootParams::new(root, geometry.root_page_base(root)));

                for (f, filter) in filters.iter().enumerate() {
                    let filter_buffer = gpu.uniform(&FilterParams {
                        radius: filter.radius,
                        spread: filter.spread,
                        threshold: filter.threshold,
                        pad: 0,
                    });
                    let output =
                        gpu.output((cases.len() * std::mem::size_of::<FilteredRayResult>()) as u64);
                    let mut group0 = slab_bindings(&topology, [0, 5, 6, 7]);
                    group0.extend(slab_bindings(&attributes, [1, 8, 9, 10]));
                    group0.push((4, meta.as_entire_binding()));
                    group0.push((11, lod.as_entire_binding()));
                    let results: Vec<FilteredRayResult> = gpu.run_variant(
                        "filtered_rays",
                        variant,
                        cases.len() as u32,
                        &[
                            (0, group0),
                            (1, vec![]),
                            (
                                2,
                                vec![
                                    (2, ray_buffer.as_entire_binding()),
                                    (4, params.as_entire_binding()),
                                    (6, filter_buffer.as_entire_binding()),
                                    (7, output.as_entire_binding()),
                                ],
                            ),
                        ],
                        &output,
                    );

                    for (ray, result) in module_rays.iter().zip(&results) {
                        let (cpu, cpu_steps) =
                            tree64::trace_filtered(&geometry, tree, ray, 0.0, f32::MAX, filter);
                        let cpu = cpu.map(|h| {
                            (
                                h.cell.x | h.cell.y << 10 | h.cell.z << 20,
                                h.lod,
                                h.material as u32,
                                h.t,
                            )
                        });
                        let gpu_hit = (result.hit != 0).then_some((
                            result.cell,
                            result.lod,
                            result.material,
                            result.t,
                        ));
                        let same = match (cpu, gpu_hit) {
                            (None, None) => true,
                            (Some(c), Some(g)) => {
                                (c.0, c.1, c.2) == (g.0, g.1, g.2)
                                    && (c.3 - g.3).abs() <= 1e-4 * c.3.abs().max(1.0)
                            }
                            _ => false,
                        };
                        if !same || cpu_steps != result.steps {
                            failures.push(format!(
                                "{} module {module}, filter {f}: CPU {cpu:?} in {cpu_steps} steps, GPU {gpu_hit:?} in {} steps\n  ray {ray:?}",
                                asset.assembly.name, result.steps
                            ));
                        }
                        if cpu.is_some_and(|c| c.1 > 0) {
                            filtered[f] += 1;
                        }
                        // shading reads the summary normal through `lod_normal`, which must
                        // decode the record as `LodSummary::unpack` does
                        if result.hit != 0 && result.lod > 0 {
                            let cpu_normal =
                                LodSummary::unpack(geometry.lod[result.record as usize]).normal;
                            if (cpu_normal - Vec3::from(result.normal)).abs().max_element() > 1e-5 {
                                failures.push(format!(
                                    "{} module {module}, filter {f}: record {} decodes to {cpu_normal} on the CPU, {:?} on the GPU",
                                    asset.assembly.name, result.record, result.normal
                                ));
                            }
                        }
                    }
                }
            }
        }
        eprintln!(
            "  {variant:?}, {} topology / {} attribute slab(s): {rays} rays per filter, filtered hits per filter {filtered:?}, {} mismatches",
            geometry.topology.len(),
            geometry.attributes.len(),
            failures.len()
        );
        assert_eq!(filtered[0], 0, "a zero cone must never filter");
        assert!(
            filtered[2] > rays / 20,
            "the wide cone should filter ({} of {rays})",
            filtered[2]
        );
        assert!(
            failures.is_empty(),
            "{} filtered traversal mismatches, first:\n{}",
            failures.len(),
            failures[0]
        );
    }
}

// INFO: -----------------------
//         scene kernel
// -----------------------------

/// The real assets in a showcase-like layout: meadow ground, a grove, a crystal grotto whose
/// crystals are rotated, finer-grained pose groups, crystal outcrops, and rotors, posed at
/// animation `time`.
fn showcase_instances(
    library: &common::world::VantaAssetLibrary,
    rng: &mut Rng,
    time: f32,
) -> Vec<voxel::scene::InstanceDesc> {
    use bevy::math::{Affine3A, Quat};
    let mut instances = Vec::new();
    let mut place = |name: &str, position: Vec3, rotation: Quat, scale: f32| {
        let index = library
            .index_of(library.id_by_name(name).expect(name))
            .unwrap();
        let assembly = &library.assets()[index].assembly;
        let phase = instances.len() as f32 * 1.7;
        let pose = voxel::PoseDriver { phase, gust: 1.0 }.evaluate(assembly, time);
        instances.push(voxel::scene::InstanceDesc {
            key: instances.len() as u64,
            asset: index as u32,
            root_to_world: Affine3A::from_scale_rotation_translation(
                Vec3::splat(assembly.voxel_size * scale),
                rotation,
                position,
            ),
            pose,
        });
    };
    for z in -2..=2 {
        for x in -2..=2 {
            let turn = Quat::from_rotation_y((x + z) as f32 * std::f32::consts::FRAC_PI_2);
            place(
                "meadow_tile",
                Vec3::new(x as f32, 0.0, z as f32) * 10.24,
                turn,
                1.0,
            );
        }
    }
    place(
        "crystal_grotto",
        Vec3::new(9.0, 0.0, -6.0),
        Quat::from_rotation_y(-2.3),
        1.0,
    );
    for i in 0..60 {
        let p = Vec3::new(rng.range(-12.0, 12.0), 0.0, rng.range(-12.0, 8.0));
        let r = Quat::from_euler(
            bevy::math::EulerRot::YXZ,
            rng.range(0.0, std::f32::consts::TAU),
            rng.range(-0.5, 0.5),
            0.0,
        );
        let scale = rng.range(0.5, 1.2);
        let name = ["oak", "crystal_cluster", "crystal_cluster", "kinetic_rotor"][i % 4];
        place(name, p, r, scale);
    }
    instances
}

/// Every scene organization with every local hierarchy.
fn scene_variants() -> Vec<(voxel::scene::SceneAcceleration, HierarchyKind)> {
    use voxel::scene::SceneAcceleration;
    SceneAcceleration::ALL
        .into_iter()
        .flat_map(|mode| HierarchyKind::ALL.map(|kind| (mode, kind)))
        .collect()
}

/// Bind groups for the `scene_rays` kernel: geometry, scene, and the test's rays and results. The
/// automatic pipeline layout only holds what an entry point reads: tracing never reads attributes
/// or materials, and the hardware broad phase (given its `tlas`) reads neither the software BVHs
/// nor the instance tables.
#[allow(clippy::too_many_arguments)]
fn scene_groups<'a>(
    topology: &'a [wgpu::Buffer],
    template_bvh: &'a wgpu::Buffer,
    meta: &'a wgpu::Buffer,
    [uniform, bvh, frames, instances, slot_frames]: [&'a wgpu::Buffer; 5],
    tlas: Option<&'a wgpu::Tlas>,
    rays: &'a wgpu::Buffer,
    output: &'a wgpu::Buffer,
) -> [Group<'a>; 3] {
    let mut geometry = topology_bindings(topology);
    let scene = match tlas {
        Some(tlas) => vec![
            (0, uniform.as_entire_binding()),
            (2, frames.as_entire_binding()),
            (5, wgpu::BindingResource::AccelerationStructure(tlas)),
        ],
        None => {
            geometry.push((3, template_bvh.as_entire_binding()));
            vec![
                (0, uniform.as_entire_binding()),
                (1, bvh.as_entire_binding()),
                (2, frames.as_entire_binding()),
                (3, instances.as_entire_binding()),
                (4, slot_frames.as_entire_binding()),
            ]
        }
    };
    geometry.push((4, meta.as_entire_binding()));
    [
        (0, geometry),
        (1, scene),
        (
            2,
            vec![
                (2, rays.as_entire_binding()),
                (3, output.as_entire_binding()),
            ],
        ),
    ]
}

/// The hardware broad phase's acceleration structure: one shared unit-box BLAS, and a TLAS
/// instance per pose frame mapping the box onto the frame's padded group-local bounds, with the
/// frame index as custom data (see `trace_scene` in `lib/scene.wesl`).
fn scene_tlas(
    gpu: &Gpu,
    snapshot: &voxel::scene::SceneSnapshot,
    templates: &[voxel::scene::AssetTemplate],
) -> wgpu::Tlas {
    let device = &gpu.device;
    let vertices: Vec<[f32; 3]> = (0..8u32)
        .map(|i| [(i & 1) as f32, (i >> 1 & 1) as f32, (i >> 2 & 1) as f32])
        .collect();
    // two triangles per face; the query culls neither winding
    let faces = [
        [0, 2, 6, 4],
        [1, 3, 7, 5],
        [0, 1, 5, 4],
        [2, 3, 7, 6],
        [0, 1, 3, 2],
        [4, 5, 7, 6],
    ];
    let indices: Vec<u32> = faces
        .iter()
        .flat_map(|[a, b, c, d]| [*a, *b, *c, *a, *c, *d])
        .collect();
    let input = |contents: &[u8]| {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents,
            usage: wgpu::BufferUsages::BLAS_INPUT,
        })
    };
    let vertex_buffer = input(bytemuck::cast_slice(&vertices));
    let index_buffer = input(bytemuck::cast_slice(&indices));

    let size = wgpu::BlasTriangleGeometrySizeDescriptor {
        vertex_format: wgpu::VertexFormat::Float32x3,
        vertex_count: vertices.len() as u32,
        index_format: Some(wgpu::IndexFormat::Uint32),
        index_count: Some(indices.len() as u32),
        // not opaque, so every face crossing is a candidate the shader sees
        flags: wgpu::AccelerationStructureGeometryFlags::empty(),
    };
    let blas = device.create_blas(
        &wgpu::CreateBlasDescriptor {
            label: Some("unit_box"),
            flags: wgpu::AccelerationStructureFlags::PREFER_FAST_TRACE,
            update_mode: wgpu::AccelerationStructureUpdateMode::Build,
        },
        wgpu::BlasGeometrySizeDescriptors::Triangles {
            descriptors: vec![size.clone()],
        },
    );

    let boxes: Vec<_> = snapshot.frame_boxes(templates).collect();
    let mut tlas = device.create_tlas(&wgpu::CreateTlasDescriptor {
        label: Some("scene_frames"),
        max_instances: boxes.len().max(1) as u32,
        flags: wgpu::AccelerationStructureFlags::PREFER_FAST_TRACE,
        update_mode: wgpu::AccelerationStructureUpdateMode::Build,
    });
    for (frame, frame_box) in boxes.iter().enumerate() {
        if let Some(frame_box) = frame_box {
            let rows = client::render::gpu::layout::affine_rows(frame_box);
            tlas[frame] = Some(wgpu::TlasInstance::new(
                &blas,
                bytemuck::cast(rows),
                frame as u32,
                0xFF,
            ));
        }
    }

    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.build_acceleration_structures(
        std::iter::once(&wgpu::BlasBuildEntry {
            blas: &blas,
            geometry: wgpu::BlasGeometries::TriangleGeometries(vec![wgpu::BlasTriangleGeometry {
                size: &size,
                vertex_buffer: &vertex_buffer,
                first_vertex: 0,
                vertex_stride: 12,
                index_buffer: Some(&index_buffer),
                first_index: Some(0),
                transform_buffer: None,
                transform_buffer_offset: None,
            }]),
        }),
        std::iter::once(&tlas),
    );
    gpu.queue.submit([encoder.finish()]);
    tlas
}

#[test]
fn gpu_scene_traversal_matches_cpu() {
    use client::render::{geometry::GpuGeometryLibrary, scene::GpuSceneData};
    use voxel::scene::{AssetTemplate, SceneSnapshot};

    let Some(gpu) = gpu() else { return };
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let library = common::world::assets::load_library(
        &workspace.join("assets/shared/vanta"),
        &std::env::temp_dir().join("vanta-conformance-cache"),
    );
    let templates: Vec<AssetTemplate> = library
        .assets()
        .iter()
        .map(|a| (*a.template).clone())
        .collect();

    let mut rng = Rng(9);
    let instances = showcase_instances(&library, &mut rng, 1.3);

    // a camera like the game's spawn view, plus rays aimed into the clutter
    let eye = Vec3::new(0.0, 1.7, 14.0);
    let mut rays: Vec<Ray> = (0..64 * 36)
        .map(|i| {
            let u = ((i % 64) as f32 + 0.5) / 64.0 * 2.0 - 1.0;
            let v = 1.0 - ((i / 64) as f32 + 0.5) / 36.0 * 2.0;
            Ray::new(eye, Vec3::new(u * 1.03, v * 0.58, -1.0).normalize())
        })
        .collect();
    rays.extend((0..2000).map(|_| {
        let target = Vec3::new(
            rng.range(-12.0, 12.0),
            rng.range(0.0, 4.0),
            rng.range(-12.0, 8.0),
        );
        let dir = rng.direction();
        Ray::new(target - dir * 15.0, dir)
    }));
    let cases: Vec<RayCase> = rays
        .iter()
        .map(|r| RayCase {
            origin: r.origin.to_array(),
            t_min: 0.0,
            dir: r.dir.to_array(),
            t_max: 3.0e38,
        })
        .collect();

    // the production layout, then one spread over several slabs: pages small enough, and
    // bindings limited enough, that topology needs more than one slab
    let unlimited = ResidencyLimits {
        max_storage_buffer_binding_size: 1 << 30,
        max_buffer_size: 1 << 30,
    };
    let single = GpuGeometryLibrary::build(&library, &unlimited).unwrap();
    let page_words = 1 << 16;
    let topology_bytes = single.stats.topology_bytes;
    let attribute_bytes = single.stats.attribute_bytes;
    let slab_bytes = (topology_bytes / 2).max(attribute_bytes.div_ceil(4)) + page_words as u64 * 4;
    let limits = ResidencyLimits {
        max_storage_buffer_binding_size: slab_bytes,
        max_buffer_size: 1 << 30,
    };
    let multi = GpuGeometryLibrary::build_with_page_words(&library, &limits, page_words).unwrap();
    assert!(
        multi.topology.len() > 1,
        "the multi-slab layout should split topology"
    );

    for geometry in [single, multi] {
        eprintln!(
            "  {} topology slab(s), {} attribute slab(s)",
            geometry.topology.len(),
            geometry.attributes.len()
        );
        // geometry group, exactly as the renderer binds it
        let meta = GpuGeometryMeta::new(
            &geometry.topology_page_entries,
            &geometry.attribute_page_entries,
        );
        let topology: Vec<wgpu::Buffer> =
            geometry.topology.iter().map(|s| gpu.storage(s)).collect();
        let template_bvh = gpu.storage(&geometry.template_bvh);
        let meta_buffer = gpu.uniform(&meta);

        for (mode, hierarchy) in scene_variants() {
            let snapshot = SceneSnapshot::build(&templates, &instances, mode, None);
            let scene = GpuSceneData::pack(&snapshot, &library, &geometry, hierarchy);
            let uniform = gpu.uniform(&scene.uniform);
            let bvh = gpu.storage(&scene.bvh);
            let frames = gpu.storage(&scene.frames);
            let instance_buffer = gpu.storage(&scene.instances);
            let slot_frames = gpu.storage(&scene.slot_frames);
            let ray_buffer = gpu.storage(&cases);
            let output = gpu.output(cases.len() as u64 * 16);

            // the software broad phase, plus the hardware one where it applies: flattened groups
            // on the single-slab fast path
            let mut variants = vec![(Variant::for_slabs(topology.len()), None)];
            if gpu.ray_query
                && mode == voxel::scene::SceneAcceleration::FlattenedGroups
                && topology.len() == 1
            {
                variants.push((
                    Variant::RayQuery,
                    Some(scene_tlas(&gpu, &snapshot, &templates)),
                ));
            }

            for (variant, tlas) in &variants {
                let results: Vec<RayResult> = gpu.run_variant(
                    "scene_rays",
                    *variant,
                    cases.len() as u32,
                    &scene_groups(
                        &topology,
                        &template_bvh,
                        &meta_buffer,
                        [&uniform, &bvh, &frames, &instance_buffer, &slot_frames],
                        tlas.as_ref(),
                        &ray_buffer,
                        &output,
                    ),
                    &output,
                );

                let mut cpu_hits = 0;
                let mut mismatches = Vec::new();
                for (ray, gpu_result) in rays.iter().zip(&results) {
                    let (cpu, _) =
                        snapshot.intersect(&templates, ray, 0.0, 3.0e38, |frame, local, t0, t1| {
                            let module = library.assets()[frame.asset as usize].assembly.groups
                                [frame.group as usize]
                                .module;
                            geometry.assets[frame.asset as usize]
                                .root(module, hierarchy)
                                .trace(&geometry, local, t0, t1)
                        });
                    cpu_hits += cpu.is_some() as usize;
                    let agree = match (cpu, gpu_result.hit != 0) {
                        (None, false) => true,
                        (Some(c), true) => {
                            (c.voxel.t - gpu_result.t).abs() <= 1e-3 * c.voxel.t.max(1.0)
                        }
                        _ => false,
                    };
                    if !agree {
                        mismatches.push(format!(
                            "cpu {:?} gpu {:?} ray {ray:?}",
                            cpu.map(|c| (c.frame, c.voxel.t)),
                            gpu_result
                        ));
                    }
                }
                eprintln!(
                    "  {mode:?} {hierarchy:?} ({variant:?}): {} rays, {cpu_hits} CPU hits, {} disagreements",
                    rays.len(),
                    mismatches.len()
                );
                assert!(cpu_hits > rays.len() / 4, "the test scene should be hit");
                // exact float ties may resolve differently, but they are vanishingly rare in this set
                assert!(
                    mismatches.len() * 1000 <= rays.len(),
                    "{} disagreements, first: {}",
                    mismatches.len(),
                    mismatches[0]
                );
            }
        }
    }
}

/// Whole-scene traversal with footprint LOD filtering (`trace_scene_filtered`) must find the CPU's
/// nearest filtered hit (`SceneSnapshot::intersect` over `VoxelRoot::trace_filtered`) for a
/// world-space cone: the same frame, cell, and level, at the same distance. Every organization and
/// hierarchy (octree frames stay exact), both residency layouts, and the hardware broad phase.
#[test]
fn gpu_filtered_scene_traversal_matches_cpu() {
    use client::render::{geometry::GpuGeometryLibrary, scene::GpuSceneData};
    use voxel::{
        lod::FootprintFilter,
        scene::{AssetTemplate, SceneAcceleration, SceneSnapshot},
    };

    let Some(gpu) = gpu() else { return };
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let library = common::world::assets::load_library(
        &workspace.join("assets/shared/vanta"),
        &std::env::temp_dir().join("vanta-conformance-cache"),
    );
    let templates: Vec<AssetTemplate> = library
        .assets()
        .iter()
        .map(|a| (*a.template).clone())
        .collect();
    let mut rng = Rng(9);
    let instances = showcase_instances(&library, &mut rng, 1.3);

    // the spawn view, plus rays aimed into the clutter from 15 m away
    let eye = Vec3::new(0.0, 1.7, 14.0);
    let mut rays: Vec<Ray> = (0..64 * 36)
        .map(|i| {
            let u = ((i % 64) as f32 + 0.5) / 64.0 * 2.0 - 1.0;
            let v = 1.0 - ((i / 64) as f32 + 0.5) / 36.0 * 2.0;
            Ray::new(eye, Vec3::new(u * 1.03, v * 0.58, -1.0).normalize())
        })
        .collect();
    rays.extend((0..2000).map(|_| {
        let target = Vec3::new(
            rng.range(-12.0, 12.0),
            rng.range(0.0, 4.0),
            rng.range(-12.0, 8.0),
        );
        let dir = rng.direction();
        Ray::new(target - dir * 15.0, dir)
    }));
    let cases: Vec<RayCase> = rays
        .iter()
        .map(|r| RayCase {
            origin: r.origin.to_array(),
            t_min: 0.0,
            dir: r.dir.to_array(),
            t_max: 3.0e38,
        })
        .collect();

    // world-space cones; the showcase's voxels are 2 to 4 cm
    let filters = [
        // exact
        FootprintFilter {
            radius: 0.0,
            spread: 0.0,
            threshold: 0.5,
        },
        // a 1280x720 pixel's cone: bricks filter only in the distance
        FootprintFilter {
            radius: 0.0,
            spread: 0.0008,
            threshold: 0.5,
        },
        // a few voxels wide nearby, with a threshold low enough that foliage stays visible
        FootprintFilter {
            radius: 0.02,
            spread: 0.004,
            threshold: 0.1,
        },
        // a mid-range, stochastic-style threshold
        FootprintFilter {
            radius: 0.01,
            spread: 0.002,
            threshold: 0.37,
        },
    ];

    let unlimited = ResidencyLimits {
        max_storage_buffer_binding_size: 1 << 30,
        max_buffer_size: 1 << 30,
    };
    let single = GpuGeometryLibrary::build(&library, &unlimited).unwrap();
    let page_words = 1 << 16;
    let slab_bytes = (single.stats.topology_bytes / 2)
        .max(single.stats.attribute_bytes.div_ceil(4))
        + page_words as u64 * 4;
    let limits = ResidencyLimits {
        max_storage_buffer_binding_size: slab_bytes,
        max_buffer_size: 1 << 30,
    };
    let multi = GpuGeometryLibrary::build_with_page_words(&library, &limits, page_words).unwrap();
    assert!(
        multi.topology.len() > 1,
        "the multi-slab layout should split topology"
    );

    for geometry in [single, multi] {
        let meta = gpu.uniform(&GpuGeometryMeta::new(
            &geometry.topology_page_entries,
            &geometry.attribute_page_entries,
        ));
        let topology: Vec<wgpu::Buffer> =
            geometry.topology.iter().map(|s| gpu.storage(s)).collect();
        let template_bvh = gpu.storage(&geometry.template_bvh);
        let lod = gpu.storage(&geometry.lod);
        let ray_buffer = gpu.storage(&cases);
        let output = gpu.output((cases.len() * std::mem::size_of::<FilteredRayResult>()) as u64);

        for (mode, hierarchy) in scene_variants() {
            let snapshot = SceneSnapshot::build(&templates, &instances, mode, None);
            let scene = GpuSceneData::pack(&snapshot, &library, &geometry, hierarchy);
            let uniform = gpu.uniform(&scene.uniform);
            let bvh = gpu.storage(&scene.bvh);
            let frames = gpu.storage(&scene.frames);
            let instance_buffer = gpu.storage(&scene.instances);
            let slot_frames = gpu.storage(&scene.slot_frames);

            let mut variants = vec![(
                if geometry.multi_slab() {
                    Variant::FootprintLodMultiSlab
                } else {
                    Variant::FootprintLod
                },
                None,
            )];
            if gpu.ray_query && mode == SceneAcceleration::FlattenedGroups && !geometry.multi_slab()
            {
                variants.push((
                    Variant::FootprintLodRayQuery,
                    Some(scene_tlas(&gpu, &snapshot, &templates)),
                ));
            }

            for (variant, tlas) in &variants {
                let mut filtered = [0usize; 4];
                for (f, filter) in filters.iter().enumerate() {
                    let filter_buffer = gpu.uniform(&FilterParams {
                        radius: filter.radius,
                        spread: filter.spread,
                        threshold: filter.threshold,
                        pad: 0,
                    });
                    let [(_, mut group0), group1, _] = scene_groups(
                        &topology,
                        &template_bvh,
                        &meta,
                        [&uniform, &bvh, &frames, &instance_buffer, &slot_frames],
                        tlas.as_ref(),
                        &ray_buffer,
                        &output,
                    );
                    group0.push((11, lod.as_entire_binding()));
                    let results: Vec<FilteredRayResult> = gpu.run_variant(
                        "filtered_scene_rays",
                        *variant,
                        cases.len() as u32,
                        &[
                            (0, group0),
                            group1,
                            (
                                2,
                                vec![
                                    (2, ray_buffer.as_entire_binding()),
                                    (6, filter_buffer.as_entire_binding()),
                                    (7, output.as_entire_binding()),
                                ],
                            ),
                        ],
                        &output,
                    );

                    let mut mismatches = Vec::new();
                    for (ray, result) in rays.iter().zip(&results) {
                        let (cpu, _) = snapshot.intersect(
                            &templates,
                            ray,
                            0.0,
                            3.0e38,
                            |frame, local, t0, t1| {
                                let module = library.assets()[frame.asset as usize].assembly.groups
                                    [frame.group as usize]
                                    .module;
                                geometry.assets[frame.asset as usize]
                                    .root(module, hierarchy)
                                    .trace_filtered(&geometry, local, t0, t1, filter)
                                    .0
                            },
                        );
                        let cpu = cpu.map(|h| {
                            (
                                h.frame,
                                h.voxel.cell.x | h.voxel.cell.y << 10 | h.voxel.cell.z << 20,
                                h.voxel.lod,
                                h.voxel.t,
                            )
                        });
                        let gpu_hit = (result.hit != 0).then_some((
                            result.frame,
                            result.cell,
                            result.lod,
                            result.t,
                        ));
                        let agree = match (cpu, gpu_hit) {
                            (None, None) => true,
                            (Some(c), Some(g)) => {
                                (c.0, c.1, c.2) == (g.0, g.1, g.2)
                                    && (c.3 - g.3).abs() <= 1e-3 * c.3.max(1.0)
                            }
                            _ => false,
                        };
                        if !agree {
                            mismatches.push(format!("cpu {cpu:?} gpu {gpu_hit:?} ray {ray:?}"));
                        }
                        if cpu.is_some_and(|c| c.2 > 0) {
                            filtered[f] += 1;
                        }
                    }
                    // exact float ties may resolve differently, but they are vanishingly rare
                    assert!(
                        mismatches.len() * 1000 <= rays.len(),
                        "{mode:?} {hierarchy:?} ({variant:?}), filter {f}: {} disagreements, first: {}",
                        mismatches.len(),
                        mismatches[0]
                    );
                    if !mismatches.is_empty() {
                        eprintln!(
                            "    filter {f}: {} tie disagreement(s), first: {}",
                            mismatches.len(),
                            mismatches[0]
                        );
                    }
                }
                eprintln!(
                    "  {} topology slab(s), {mode:?} {hierarchy:?} ({variant:?}): {} rays, filtered hits per filter {filtered:?}",
                    geometry.topology.len(),
                    rays.len()
                );
                assert_eq!(filtered[0], 0, "a zero cone must never filter");
                match hierarchy {
                    HierarchyKind::Tree64 => assert!(
                        filtered[2] > rays.len() / 20,
                        "the wide cone should filter ({} of {})",
                        filtered[2],
                        rays.len()
                    ),
                    HierarchyKind::Octree => {
                        assert_eq!(filtered, [0; 4], "the octree is never filtered")
                    }
                }
            }
        }
    }
}

/// Motion reconstruction on the real adapter: from each hit's distance and resolved frame, the
/// GPU must recover the point's previous world position exactly as the CPU does, across a pose
/// change between two snapshots.
#[test]
fn gpu_motion_reconstruction_matches_cpu() {
    use client::render::{geometry::GpuGeometryLibrary, scene::GpuSceneData};
    use voxel::scene::{AssetTemplate, SceneAcceleration, SceneSnapshot};

    let Some(gpu) = gpu() else { return };
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let library = common::world::assets::load_library(
        &workspace.join("assets/shared/vanta"),
        &std::env::temp_dir().join("vanta-conformance-cache"),
    );
    let limits = ResidencyLimits {
        max_storage_buffer_binding_size: 1 << 30,
        max_buffer_size: 1 << 30,
    };
    let geometry = GpuGeometryLibrary::build(&library, &limits).unwrap();
    let templates: Vec<AssetTemplate> = library
        .assets()
        .iter()
        .map(|a| (*a.template).clone())
        .collect();

    // the same scene a frame and a bit apart, so swaying and spinning groups really move
    let before = showcase_instances(&library, &mut Rng(9), 1.25);
    let after = showcase_instances(&library, &mut Rng(9), 1.3);
    let mode = SceneAcceleration::TemplateMiniBvh;
    let previous = SceneSnapshot::build(&templates, &before, mode, None);
    let snapshot = SceneSnapshot::build(&templates, &after, mode, Some(&previous));
    let scene = GpuSceneData::pack(&snapshot, &library, &geometry, HierarchyKind::Tree64);

    let mut rng = Rng(31);
    let rays: Vec<Ray> = (0..4000)
        .map(|_| {
            let target = Vec3::new(
                rng.range(-12.0, 12.0),
                rng.range(0.0, 4.0),
                rng.range(-12.0, 8.0),
            );
            let dir = rng.direction();
            Ray::new(target - dir * 15.0, dir)
        })
        .collect();
    let cases: Vec<RayCase> = rays
        .iter()
        .map(|r| RayCase {
            origin: r.origin.to_array(),
            t_min: 0.0,
            dir: r.dir.to_array(),
            t_max: 3.0e38,
        })
        .collect();

    let topology: Vec<wgpu::Buffer> = geometry.topology.iter().map(|s| gpu.storage(s)).collect();
    let template_bvh = gpu.storage(&geometry.template_bvh);
    let meta = gpu.uniform(&GpuGeometryMeta::new(
        &geometry.topology_page_entries,
        &geometry.attribute_page_entries,
    ));
    let uniform = gpu.uniform(&scene.uniform);
    let bvh = gpu.storage(&scene.bvh);
    let frames = gpu.storage(&scene.frames);
    let instance_buffer = gpu.storage(&scene.instances);
    let slot_frames = gpu.storage(&scene.slot_frames);
    let ray_buffer = gpu.storage(&cases);
    let output = gpu.output(cases.len() as u64 * 16);
    let results: Vec<[f32; 4]> = gpu.run(
        "scene_motion",
        cases.len() as u32,
        &[
            (0, {
                let mut bindings = topology_bindings(&topology);
                bindings.push((3, template_bvh.as_entire_binding()));
                bindings.push((4, meta.as_entire_binding()));
                bindings
            }),
            (
                1,
                vec![
                    (0, uniform.as_entire_binding()),
                    (1, bvh.as_entire_binding()),
                    (2, frames.as_entire_binding()),
                    (3, instance_buffer.as_entire_binding()),
                    (4, slot_frames.as_entire_binding()),
                ],
            ),
            (
                2,
                vec![
                    (2, ray_buffer.as_entire_binding()),
                    (5, output.as_entire_binding()),
                ],
            ),
        ],
        &output,
    );

    let (mut hits, mut moving, mut mismatches) = (0, 0, Vec::new());
    for (ray, gpu_result) in rays.iter().zip(&results) {
        let (cpu, _) = snapshot.intersect(&templates, ray, 0.0, 3.0e38, |frame, local, t0, t1| {
            let module =
                library.assets()[frame.asset as usize].assembly.groups[frame.group as usize].module;
            geometry.assets[frame.asset as usize]
                .root(module, HierarchyKind::Tree64)
                .trace(&geometry, local, t0, t1)
        });
        let expected = cpu
            .map(|hit| snapshot.frames[hit.frame as usize].previous_position(ray.at(hit.voxel.t)));
        let gpu_previous =
            (gpu_result[3] == 1.0).then(|| Vec3::new(gpu_result[0], gpu_result[1], gpu_result[2]));
        let agree = match (expected, gpu_previous) {
            (None, None) => true,
            (Some(e), Some(g)) => {
                hits += 1;
                moving += (e.distance(ray.at(cpu.unwrap().voxel.t)) > 1e-4) as u32;
                e.distance(g) <= 1e-3 * (1.0 + e.length())
            }
            _ => false,
        };
        if !agree {
            mismatches.push(format!("cpu {expected:?} gpu {gpu_previous:?} ray {ray:?}"));
        }
    }
    eprintln!(
        "  {} rays, {hits} hits, {moving} on moving groups, {} disagreements",
        rays.len(),
        mismatches.len()
    );
    assert!(hits > rays.len() / 4, "the test scene should be hit");
    assert!(moving > 20, "some hits should be on animated groups");
    // exact float ties may resolve differently, but they are vanishingly rare in this set
    assert!(
        mismatches.len() * 1000 <= rays.len(),
        "{} disagreements, first: {}",
        mismatches.len(),
        mismatches[0]
    );
}

/// A dense grove of hundreds of animated, varied pose instances: the spec's workload for comparing
/// template-shared, per-instance refit, and flattened pose-group organization.
fn grove_instances(library: &common::world::VantaAssetLibrary) -> Vec<voxel::scene::InstanceDesc> {
    use bevy::math::{Affine3A, Quat};
    (0..300)
        .map(|i| {
            let name = ["oak", "oak", "kinetic_rotor"][i % 3];
            let index = library
                .index_of(library.id_by_name(name).expect(name))
                .unwrap();
            let assembly = &library.assets()[index].assembly;
            let angle = i as f32 * 2.399;
            let position = Vec3::new(angle.cos(), 0.0, angle.sin()) * (i as f32).sqrt() * 2.2;
            voxel::scene::InstanceDesc {
                key: i as u64,
                asset: index as u32,
                root_to_world: Affine3A::from_scale_rotation_translation(
                    Vec3::splat(assembly.voxel_size * (0.7 + 0.1 * (i % 5) as f32)),
                    Quat::from_rotation_y(angle),
                    position,
                ),
                pose: voxel::PoseDriver {
                    phase: angle,
                    gust: 1.0,
                }
                .evaluate(assembly, 1.3),
            }
        })
        .collect()
}

/// Primary-ray throughput of the production scene kernel on the real assets, plus the search
/// statistics that explain it. Not a correctness test; run with
/// `just test-gpu gpu_scene_throughput --ignored` (ideally with `--release`) to compare changes.
#[test]
#[ignore]
fn gpu_scene_throughput() {
    use client::render::{geometry::GpuGeometryLibrary, scene::GpuSceneData};
    use voxel::scene::{AssetTemplate, SceneAcceleration, SceneSnapshot, SceneTraceStats};

    let Some(gpu) = gpu() else { return };
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let library = common::world::assets::load_library(
        &workspace.join("assets/shared/vanta"),
        &std::env::temp_dir().join("vanta-conformance-cache"),
    );
    let limits = voxel::residency::ResidencyLimits {
        max_storage_buffer_binding_size: 1 << 30,
        max_buffer_size: 1 << 30,
    };
    let geometry = GpuGeometryLibrary::build(&library, &limits).unwrap();
    let templates: Vec<AssetTemplate> = library
        .assets()
        .iter()
        .map(|a| (*a.template).clone())
        .collect();

    let (width, height) = (1280u32, 720u32);
    let camera = |eye: Vec3, target: Vec3| -> Vec<Ray> {
        let forward = (target - eye).normalize();
        let right = forward.cross(Vec3::Y).normalize();
        let up = right.cross(forward);
        (0..width * height)
            .map(|i| {
                let u = ((i % width) as f32 + 0.5) / width as f32 * 2.0 - 1.0;
                let v = 1.0 - ((i / width) as f32 + 0.5) / height as f32 * 2.0;
                Ray::new(
                    eye,
                    (forward + right * u * 1.03 + up * v * 0.58).normalize(),
                )
            })
            .collect()
    };
    let scenes = [
        (
            "showcase",
            showcase_instances(&library, &mut Rng(9), 1.3),
            camera(Vec3::new(0.0, 1.7, 14.0), Vec3::new(0.0, 1.7, 0.0)),
        ),
        (
            "grove of 300",
            grove_instances(&library),
            camera(Vec3::new(0.0, 6.0, 48.0), Vec3::new(0.0, 4.0, 0.0)),
        ),
    ];

    let meta = gpu.uniform(&GpuGeometryMeta::new(
        &geometry.topology_page_entries,
        &geometry.attribute_page_entries,
    ));
    let topology: Vec<wgpu::Buffer> = geometry.topology.iter().map(|s| gpu.storage(s)).collect();
    let template_bvh = gpu.storage(&geometry.template_bvh);
    let template_bytes = geometry.template_bvh.len() * std::mem::size_of::<voxel::Bvh4Node>();
    let lod = gpu.storage(&geometry.lod);
    // footprint filtering's cones: zero (what the variant costs when nothing filters) and a
    // 1280x720 pixel's (radius 0, spread of half a pixel's angle)
    let cones = [("zero cone", 0.0), ("pixel cone", 0.0008)].map(|(name, spread)| {
        (
            name,
            gpu.uniform(&FilterParams {
                radius: 0.0,
                spread,
                threshold: 0.5,
                pad: 0,
            }),
        )
    });

    for (scene_name, instances, rays) in &scenes {
        eprintln!("  {scene_name}: {} instances", instances.len());
        let cases: Vec<RayCase> = rays
            .iter()
            .map(|r| RayCase {
                origin: r.origin.to_array(),
                t_min: 0.0,
                dir: r.dir.to_array(),
                t_max: 3.0e38,
            })
            .collect();
        let ray_buffer = gpu.storage(&cases);
        let output = gpu.output(cases.len() as u64 * 16);
        let filtered_output =
            gpu.output((cases.len() * std::mem::size_of::<FilteredRayResult>()) as u64);

        for mode in SceneAcceleration::ALL {
            let build_start = std::time::Instant::now();
            let snapshot = SceneSnapshot::build(&templates, instances, mode, None);
            let build_ms = build_start.elapsed().as_secs_f64() * 1000.0;

            // search statistics from the CPU reference on a subsample of the same rays
            let mut totals = SceneTraceStats::default();
            let sample: Vec<&Ray> = rays.iter().step_by(97).collect();
            for ray in &sample {
                let (_, stats) =
                    snapshot.intersect(&templates, ray, 0.0, 3.0e38, |frame, local, t0, t1| {
                        let module = library.assets()[frame.asset as usize].assembly.groups
                            [frame.group as usize]
                            .module;
                        geometry.assets[frame.asset as usize]
                            .root(module, HierarchyKind::Tree64)
                            .trace(&geometry, local, t0, t1)
                    });
                totals.scene_nodes += stats.scene_nodes;
                totals.mini_bvh_nodes += stats.mini_bvh_nodes;
                totals.group_candidates += stats.group_candidates;
            }
            let per_ray = |n: u32| n as f64 / sample.len() as f64;
            let node_bytes = std::mem::size_of::<voxel::Bvh4Node>();
            let mini_bvh_bytes = match mode {
                SceneAcceleration::TemplateMiniBvh => template_bytes,
                SceneAcceleration::FlattenedGroups => 0,
                SceneAcceleration::InstanceRefit => {
                    snapshot
                        .instance_bvhs
                        .iter()
                        .map(|b| b.nodes.len())
                        .sum::<usize>()
                        * node_bytes
                }
            };
            eprintln!(
                "    {mode:?}: scene BVH {:.1} KiB, mini-BVHs {:.1} KiB, snapshot build {build_ms:.2} ms; per ray {:.1} scene nodes, {:.1} mini-BVH nodes, {:.2} group candidates",
                (snapshot.bvh.nodes.len() * node_bytes) as f64 / 1024.0,
                mini_bvh_bytes as f64 / 1024.0,
                per_ray(totals.scene_nodes),
                per_ray(totals.mini_bvh_nodes),
                per_ray(totals.group_candidates),
            );

            // the hardware broad phase replaces the flattened scene BVH with a TLAS of frames
            let tlas = (gpu.ray_query && mode == SceneAcceleration::FlattenedGroups).then(|| {
                let start = std::time::Instant::now();
                let tlas = scene_tlas(&gpu, &snapshot, &templates);
                gpu.wait();
                eprintln!(
                    "      TLAS of {} frames built in {:.2} ms (CPU instance writes + GPU build)",
                    snapshot.frames.len(),
                    start.elapsed().as_secs_f64() * 1000.0
                );
                tlas
            });
            // `(hierarchy, kernels, footprint cone)`: filtered kernels run `filtered_scene_rays`
            let mut variants = vec![
                (HierarchyKind::Tree64, Variant::Default, None),
                (HierarchyKind::Tree64, Variant::Stackless, None),
                (HierarchyKind::Tree64, Variant::FootprintLod, Some(0)),
                (HierarchyKind::Tree64, Variant::FootprintLod, Some(1)),
                (HierarchyKind::Octree, Variant::Default, None),
            ];
            if tlas.is_some() {
                variants.extend([
                    (HierarchyKind::Tree64, Variant::RayQuery, None),
                    (
                        HierarchyKind::Tree64,
                        Variant::FootprintLodRayQuery,
                        Some(0),
                    ),
                    (
                        HierarchyKind::Tree64,
                        Variant::FootprintLodRayQuery,
                        Some(1),
                    ),
                    (HierarchyKind::Octree, Variant::RayQuery, None),
                ]);
            }

            for (hierarchy, variant, cone) in variants {
                let scene = GpuSceneData::pack(&snapshot, &library, &geometry, hierarchy);
                let uniform = gpu.uniform(&scene.uniform);
                let bvh = gpu.storage(&scene.bvh);
                let frames = gpu.storage(&scene.frames);
                let instance_buffer = gpu.storage(&scene.instances);
                let slot_frames = gpu.storage(&scene.slot_frames);
                let hardware = matches!(variant, Variant::RayQuery | Variant::FootprintLodRayQuery);
                let mut groups = scene_groups(
                    &topology,
                    &template_bvh,
                    &meta,
                    [&uniform, &bvh, &frames, &instance_buffer, &slot_frames],
                    tlas.as_ref().filter(|_| hardware),
                    &ray_buffer,
                    &output,
                );
                let (entry, label, output) = match cone {
                    Some(c) => {
                        let (name, filter) = &cones[c];
                        groups[0].1.push((11, lod.as_entire_binding()));
                        groups[2].1 = vec![
                            (2, ray_buffer.as_entire_binding()),
                            (6, filter.as_entire_binding()),
                            (7, filtered_output.as_entire_binding()),
                        ];
                        ("filtered_scene_rays", format!(", {name}"), &filtered_output)
                    }
                    None => ("scene_rays", String::new(), &output),
                };
                let run = || -> Vec<u32> {
                    gpu.run_variant(entry, variant, cases.len() as u32, &groups, output)
                };
                // warm up (pipeline creation, caches), then time
                run();
                let start = std::time::Instant::now();
                let runs = 5;
                for _ in 0..runs {
                    run();
                }
                let ms = start.elapsed().as_secs_f64() * 1000.0 / runs as f64;
                let rays = cases.len() as f64;
                match gpu.gpu_ms(entry, variant, cases.len() as u32, &groups, 20) {
                    Some(gpu_ms) => eprintln!(
                        "      {hierarchy:?} ({variant:?}{label}): {width}x{height} primary rays in {gpu_ms:.2} ms GPU ({:.0} Mrays/s), {ms:.1} ms with readback",
                        rays / gpu_ms / 1000.0
                    ),
                    None => eprintln!(
                        "      {hierarchy:?} ({variant:?}{label}): {width}x{height} primary rays in {ms:.1} ms ({:.1} Mrays/s, includes readback)",
                        rays / ms / 1000.0
                    ),
                }
            }
        }
    }
}

/// What footprint LOD filtering costs and saves in the production `PrimaryVisibility` kernels:
/// the exact and `footprint_lod` builds of `primary_visibility` traced over showcase and grove
/// views at render scales 0.5 and 1.0 of a 1280x720 target, with one- and four-pixel cones, on the
/// hardware broad phase (where the adapter has it) and the software BVH4 (flattened groups, the
/// runtime default organization). Reports kernel time from timestamp queries and, from the
/// `stats` and `vis` textures, voxel DDA steps per pixel and the share of pixels with filtered
/// hits. Not a correctness test; run with
/// `just test-gpu gpu_primary_visibility_cost --ignored --release`, one GPU benchmark at a time.
#[test]
#[ignore]
fn gpu_primary_visibility_cost() {
    use bevy::math::Mat4;
    use client::render::{
        geometry::GpuGeometryLibrary, gpu::layout::GpuViewUniform, scene::GpuSceneData,
    };
    use voxel::scene::{AssetTemplate, SceneAcceleration, SceneSnapshot};

    let Some(gpu) = gpu() else { return };
    if !gpu.timestamps {
        eprintln!("skipping: the adapter has no timestamp queries");
        return;
    }
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let library = common::world::assets::load_library(
        &workspace.join("assets/shared/vanta"),
        &std::env::temp_dir().join("vanta-conformance-cache"),
    );
    let limits = ResidencyLimits {
        max_storage_buffer_binding_size: 1 << 30,
        max_buffer_size: 1 << 30,
    };
    let geometry = GpuGeometryLibrary::build(&library, &limits).unwrap();
    assert_eq!(
        geometry.topology.len(),
        1,
        "the benchmark runs the single-slab kernels"
    );
    let templates: Vec<AssetTemplate> = library
        .assets()
        .iter()
        .map(|a| (*a.template).clone())
        .collect();

    // `(broad phase, uses the TLAS, exact build, footprint LOD build)` of the production pass
    let module = |label: &str, source: &str| {
        gpu.device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            })
    };
    let mut kernels = vec![(
        "software BVH4",
        false,
        module("primary_visibility", PRIMARY_VISIBILITY),
        module(
            "primary_visibility_footprint_lod",
            PRIMARY_VISIBILITY_FOOTPRINT_LOD,
        ),
    )];
    if gpu.ray_query {
        kernels.insert(
            0,
            (
                "hardware broad phase",
                true,
                module("primary_visibility_ray_query", PRIMARY_VISIBILITY_RAY_QUERY),
                module(
                    "primary_visibility_footprint_lod_ray_query",
                    PRIMARY_VISIBILITY_FOOTPRINT_LOD_RAY_QUERY,
                ),
            ),
        );
    }

    let topology: Vec<wgpu::Buffer> = geometry.topology.iter().map(|s| gpu.storage(s)).collect();
    let template_bvh = gpu.storage(&geometry.template_bvh);
    let lod = gpu.storage(&geometry.lod);
    let meta = gpu.uniform(&GpuGeometryMeta::new(
        &geometry.topology_page_entries,
        &geometry.attribute_page_entries,
    ));
    // `scene_groups` wants a group 2; the pass replaces it with its view and targets
    let unused = gpu.output(16);

    let build = |instances: &[voxel::scene::InstanceDesc]| {
        SceneSnapshot::build(
            &templates,
            instances,
            SceneAcceleration::FlattenedGroups,
            None,
        )
    };
    let showcase = build(&showcase_instances(&library, &mut Rng(9), 1.3));
    let grove = build(&grove_instances(&library));
    // `(scene, view, snapshot, eye, target)`: the throughput benchmark's views, a grass-level
    // diagonal across the meadow, and an elevated overview, where distance makes cones wide
    let views = [
        (
            "showcase",
            "street",
            &showcase,
            Vec3::new(0.0, 1.7, 14.0),
            Vec3::new(0.0, 1.7, 0.0),
        ),
        (
            "showcase",
            "grass level",
            &showcase,
            Vec3::new(-14.0, 0.6, 12.0),
            Vec3::new(10.0, 0.5, -10.0),
        ),
        (
            "showcase",
            "overview",
            &showcase,
            Vec3::new(0.0, 12.0, 26.0),
            Vec3::new(0.0, 0.0, -4.0),
        ),
        (
            "grove of 300",
            "distant",
            &grove,
            Vec3::new(0.0, 6.0, 48.0),
            Vec3::new(0.0, 4.0, 0.0),
        ),
    ];
    let (target_width, target_height) = (1280u32, 720u32);

    for (scene_name, view_name, snapshot, eye, target) in views {
        let scene = GpuSceneData::pack(snapshot, &library, &geometry, HierarchyKind::Tree64);
        let uniform = gpu.uniform(&scene.uniform);
        let bvh = gpu.storage(&scene.bvh);
        let frames = gpu.storage(&scene.frames);
        let instance_buffer = gpu.storage(&scene.instances);
        let slot_frames = gpu.storage(&scene.slot_frames);
        let tlas = gpu
            .ray_query
            .then(|| scene_tlas(&gpu, snapshot, &templates));
        eprintln!(
            "  {scene_name}, {view_name} view ({} pose frames)",
            snapshot.frames.len()
        );

        for render_scale in [0.5f32, 1.0] {
            let width = (target_width as f32 * render_scale) as u32;
            let height = (target_height as f32 * render_scale) as u32;
            let texture = |format: wgpu::TextureFormat| {
                gpu.device.create_texture(&wgpu::TextureDescriptor {
                    label: None,
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
                    view_formats: &[],
                })
            };
            let vis = texture(wgpu::TextureFormat::Rg32Uint);
            let depth = texture(wgpu::TextureFormat::R32Float);
            let cell = texture(wgpu::TextureFormat::R32Uint);
            let stats = texture(wgpu::TextureFormat::R32Uint);
            let [vis_view, depth_view, cell_view, stats_view] =
                [&vis, &depth, &cell, &stats].map(|t| t.create_view(&Default::default()));

            for (kernel_name, hardware, exact, filtered) in &kernels {
                // `(label, footprint scale)`: `None` is the exact build
                for (cone, scale) in [
                    ("exact", None),
                    ("1 px cone", Some(1.0)),
                    ("4 px cone", Some(4.0)),
                ] {
                    let clip_from_world = Mat4::perspective_infinite_reverse_rh(
                        60f32.to_radians(),
                        target_width as f32 / target_height as f32,
                        0.05,
                    ) * Mat4::look_at_rh(eye, target, Vec3::Y);
                    let view = gpu.uniform(&GpuViewUniform {
                        world_from_clip: clip_from_world.inverse().to_cols_array(),
                        clip_from_world: clip_from_world.to_cols_array(),
                        prev_clip_from_world: clip_from_world.to_cols_array(),
                        camera_position: eye.to_array(),
                        viewport: [width as f32, height as f32],
                        target_to_grid: render_scale,
                        footprint_threshold: 0.5,
                        footprint_scale: scale.unwrap_or(1.0),
                        ..Default::default()
                    });
                    let mut groups = scene_groups(
                        &topology,
                        &template_bvh,
                        &meta,
                        [&uniform, &bvh, &frames, &instance_buffer, &slot_frames],
                        tlas.as_ref().filter(|_| *hardware),
                        &unused,
                        &unused,
                    );
                    if scale.is_some() {
                        groups[0].1.push((11, lod.as_entire_binding()));
                    }
                    groups[2].1 = vec![
                        (0, view.as_entire_binding()),
                        (1, wgpu::BindingResource::TextureView(&vis_view)),
                        (2, wgpu::BindingResource::TextureView(&depth_view)),
                        (3, wgpu::BindingResource::TextureView(&cell_view)),
                        (4, wgpu::BindingResource::TextureView(&stats_view)),
                    ];
                    let module = if scale.is_some() { filtered } else { exact };
                    let (pipeline, bind_groups) = gpu.bind_module(module, "main", &groups);
                    let workgroups = [width.div_ceil(8), height.div_ceil(8), 1];
                    // warm up (pipeline creation, caches), then time
                    gpu.time_dispatch(&pipeline, &bind_groups, workgroups, 5);
                    let ms = gpu
                        .time_dispatch(&pipeline, &bind_groups, workgroups, 50)
                        .unwrap();

                    // the last dispatch's outputs: DDA steps and filtered hits per pixel
                    let read_texture = |texture: &wgpu::Texture, texel_bytes: u32| -> Vec<u32> {
                        let bytes_per_row = width * texel_bytes;
                        assert_eq!(bytes_per_row % wgpu::COPY_BYTES_PER_ROW_ALIGNMENT, 0);
                        let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                            label: None,
                            size: (bytes_per_row * height) as u64,
                            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                            mapped_at_creation: false,
                        });
                        let mut encoder = gpu.device.create_command_encoder(&Default::default());
                        encoder.copy_texture_to_buffer(
                            wgpu::TexelCopyTextureInfo {
                                texture,
                                mip_level: 0,
                                origin: wgpu::Origin3d::ZERO,
                                aspect: wgpu::TextureAspect::All,
                            },
                            wgpu::TexelCopyBufferInfo {
                                buffer: &readback,
                                layout: wgpu::TexelCopyBufferLayout {
                                    offset: 0,
                                    bytes_per_row: Some(bytes_per_row),
                                    rows_per_image: Some(height),
                                },
                            },
                            texture.size(),
                        );
                        gpu.queue.submit([encoder.finish()]);
                        gpu.read(&readback)
                    };
                    let stats_texels = read_texture(&stats, 4);
                    let vis_texels = read_texture(&vis, 8);
                    let pixels = (width * height) as f64;
                    let steps: u64 = stats_texels.iter().map(|s| (s >> 16) as u64).sum();
                    let (mut hits, mut filtered_hits) = (0u32, 0u32);
                    for texel in vis_texels.chunks(2) {
                        if texel[0] != u32::MAX {
                            hits += 1;
                            filtered_hits += (texel[0] >> 9 & 7 != 0) as u32;
                        }
                    }
                    eprintln!(
                        "    {width}x{height} (scale {render_scale}), {kernel_name}, {cone}: {ms:.3} ms GPU, {:.1} DDA steps/px, {:.1}% hit, {:.2}% of hits filtered",
                        steps as f64 / pixels,
                        hits as f64 / pixels * 100.0,
                        filtered_hits as f64 / hits.max(1) as f64 * 100.0,
                    );
                }
            }
        }
    }
}

/// What the multi-slab path costs: the grove traced with production geometry and the
/// single-slab kernels, then with the same geometry spread over several slabs and the multi-slab
/// kernels. Run with `just test-gpu gpu_multi_slab_cost --ignored` (ideally with `--release`).
#[test]
#[ignore]
fn gpu_multi_slab_cost() {
    use client::render::{geometry::GpuGeometryLibrary, scene::GpuSceneData};
    use voxel::scene::{AssetTemplate, SceneAcceleration, SceneSnapshot};

    let Some(gpu) = gpu() else { return };
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let library = common::world::assets::load_library(
        &workspace.join("assets/shared/vanta"),
        &std::env::temp_dir().join("vanta-conformance-cache"),
    );
    let templates: Vec<AssetTemplate> = library
        .assets()
        .iter()
        .map(|a| (*a.template).clone())
        .collect();
    let instances = grove_instances(&library);

    let unlimited = ResidencyLimits {
        max_storage_buffer_binding_size: 1 << 30,
        max_buffer_size: 1 << 30,
    };
    let single = GpuGeometryLibrary::build(&library, &unlimited).unwrap();
    let page_words = 1 << 16;
    let slab_bytes = (single.stats.topology_bytes / 2)
        .max(single.stats.attribute_bytes.div_ceil(4))
        + page_words as u64 * 4;
    let limits = ResidencyLimits {
        max_storage_buffer_binding_size: slab_bytes,
        max_buffer_size: 1 << 30,
    };
    let multi = GpuGeometryLibrary::build_with_page_words(&library, &limits, page_words).unwrap();

    let (width, height) = (1280u32, 720u32);
    let eye = Vec3::new(0.0, 6.0, 48.0);
    let cases: Vec<RayCase> = (0..width * height)
        .map(|i| {
            let u = ((i % width) as f32 + 0.5) / width as f32 * 2.0 - 1.0;
            let v = 1.0 - ((i / width) as f32 + 0.5) / height as f32 * 2.0;
            let dir = Vec3::new(u * 1.03, v * 0.58 - 0.04, -1.0).normalize();
            RayCase {
                origin: eye.to_array(),
                t_min: 0.0,
                dir: dir.to_array(),
                t_max: 3.0e38,
            }
        })
        .collect();
    let ray_buffer = gpu.storage(&cases);
    let output = gpu.output(cases.len() as u64 * 16);

    for (name, geometry) in [("single slab", &single), ("multi slab", &multi)] {
        eprintln!(
            "  {name}: {} topology slab(s), {} attribute slab(s)",
            geometry.topology.len(),
            geometry.attributes.len()
        );
        let topology: Vec<wgpu::Buffer> =
            geometry.topology.iter().map(|s| gpu.storage(s)).collect();
        let template_bvh = gpu.storage(&geometry.template_bvh);
        let meta = gpu.uniform(&GpuGeometryMeta::new(
            &geometry.topology_page_entries,
            &geometry.attribute_page_entries,
        ));
        for mode in [
            SceneAcceleration::FlattenedGroups,
            SceneAcceleration::TemplateMiniBvh,
        ] {
            let snapshot = SceneSnapshot::build(&templates, &instances, mode, None);
            let scene = GpuSceneData::pack(&snapshot, &library, geometry, HierarchyKind::Tree64);
            let uniform = gpu.uniform(&scene.uniform);
            let bvh = gpu.storage(&scene.bvh);
            let frames = gpu.storage(&scene.frames);
            let instance_buffer = gpu.storage(&scene.instances);
            let slot_frames = gpu.storage(&scene.slot_frames);
            let groups = [
                (0, {
                    let mut bindings = topology_bindings(&topology);
                    bindings.push((3, template_bvh.as_entire_binding()));
                    bindings.push((4, meta.as_entire_binding()));
                    bindings
                }),
                (
                    1,
                    vec![
                        (0, uniform.as_entire_binding()),
                        (1, bvh.as_entire_binding()),
                        (2, frames.as_entire_binding()),
                        (3, instance_buffer.as_entire_binding()),
                        (4, slot_frames.as_entire_binding()),
                    ],
                ),
                (
                    2,
                    vec![
                        (2, ray_buffer.as_entire_binding()),
                        (3, output.as_entire_binding()),
                    ],
                ),
            ];
            let run = || -> Vec<RayResult> {
                gpu.run_variant(
                    "scene_rays",
                    Variant::for_slabs(geometry.topology.len()),
                    cases.len() as u32,
                    &groups,
                    &output,
                )
            };
            run();
            let start = std::time::Instant::now();
            let runs = 5;
            for _ in 0..runs {
                run();
            }
            let ms = start.elapsed().as_secs_f64() * 1000.0 / runs as f64;
            let variant = Variant::for_slabs(geometry.topology.len());
            match gpu.gpu_ms("scene_rays", variant, cases.len() as u32, &groups, 20) {
                Some(gpu_ms) => {
                    eprintln!("    {mode:?} Tree64: {gpu_ms:.2} ms GPU, {ms:.1} ms with readback")
                }
                None => eprintln!("    {mode:?} Tree64: {ms:.1} ms (includes readback)"),
            }
        }
    }
}
