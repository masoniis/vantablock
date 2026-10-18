//! Holds the production WGSL to the CPU reference.
//!
//! - Every voxel shader must parse and validate with naga, no GPU needed.
//! - On a real adapter, the WGSL mask prefix count must match the CPU bit-for-bit for every child
//!   index of every test mask, and the WGSL traversal of both local hierarchies (64-tree and
//!   octree) must satisfy the same acceptance rules as the CPU traversal (`voxel::reference`) on
//!   boundary torture rays, for both the single-page fast path and the multi-page page-table path.
//! - Whole-scene GPU traversal must find the CPU's nearest hits for every scene organization and
//!   hierarchy.
//!
//! GPU checks skip (and say so) when no adapter is available, e.g. on headless CI runners.

use bevy::{
    math::{UVec3, Vec3},
    tasks::block_on,
};
use bytemuck::{Pod, Zeroable};
use client::render::gpu::layout::{GpuGeometryMeta, root_flags};
use voxel::{
    ArenaHandle, GeometryArenas, GeometrySource, HierarchyKind, Mask64, Ray, VoxelModule,
    VoxelRoot,
    hierarchy::ATTRIBUTE_MATERIAL_MASK,
    reference::check_hit,
    residency::{ResidencyLimits, plan_slabs},
};
use wesl::include_wesl;
use wgpu::util::DeviceExt;

const PRIMARY_VISIBILITY: &str = include_wesl!("primary_visibility");
const DIRECT_LIGHTING: &str = include_wesl!("direct_lighting");
const CONFORMANCE: &str = include_wesl!("voxel_conformance");
const PRIMARY_VISIBILITY_MULTI_SLAB: &str = include_wesl!("primary_visibility_multi_slab");
const DIRECT_LIGHTING_MULTI_SLAB: &str = include_wesl!("direct_lighting_multi_slab");
const CONFORMANCE_MULTI_SLAB: &str = include_wesl!("voxel_conformance_multi_slab");
const CONFORMANCE_STACKLESS: &str = include_wesl!("voxel_conformance_stackless");

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
    ] {
        let module = naga::front::wgsl::parse_str(source)
            .unwrap_or_else(|e| panic!("{name} failed to parse:\n{}", e.emit_to_string(source)));
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::default(),
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
    /// Conformance kernels, indexed by [`Variant`].
    modules: [wgpu::ShaderModule; 3],
}

/// Bindings for one bind group: `(group, [(binding, resource)])`.
type Group<'a> = (u32, Vec<(u32, wgpu::BindingResource<'a>)>);

fn gpu() -> Option<Gpu> {
    let instance = wgpu::Instance::default();
    let adapter = match block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
    {
        Ok(adapter) => adapter,
        Err(e) => {
            eprintln!("skipping GPU conformance: no adapter ({e})");
            return None;
        }
    };
    let info = adapter.get_info();
    eprintln!("GPU conformance on {} ({:?})", info.name, info.backend);
    let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("conformance"),
        required_limits: adapter.limits(),
        ..Default::default()
    }))
    .expect("device");
    let modules = [
        ("voxel_conformance", CONFORMANCE),
        ("voxel_conformance_multi_slab", CONFORMANCE_MULTI_SLAB),
        ("voxel_conformance_stackless", CONFORMANCE_STACKLESS),
    ]
    .map(|(label, source)| {
        device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(label),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        })
    });
    Some(Gpu {
        device,
        queue,
        modules,
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
        let pipeline = self
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: None,
                module: &self.modules[variant as usize],
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

        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("poll");
        bytemuck::cast_slice(&slice.get_mapped_range()).to_vec()
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
    let params = RootParams {
        node: root.node().raw(),
        levels: root.levels() as u32,
        page_base: plan.page_entries[root.node().page() as usize],
        flags: root_flags(root),
    };
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

// INFO: -----------------------
//         scene kernel
// -----------------------------

/// The real assets in a showcase-like layout: meadow ground, a grove, a crystal grotto whose
/// crystals are rotated, finer-grained pose groups, crystal outcrops, and rotors.
fn showcase_instances(
    library: &common::world::VantaAssetLibrary,
    rng: &mut Rng,
) -> Vec<voxel::scene::InstanceDesc> {
    use bevy::math::{Affine3A, Quat};
    let mut instances = Vec::new();
    let mut place = |name: &str, position: Vec3, rotation: Quat, scale: f32| {
        let index = library
            .index_of(library.id_by_name(name).expect(name))
            .unwrap();
        let assembly = &library.assets()[index].assembly;
        let phase = instances.len() as f32 * 1.7;
        let pose = voxel::PoseDriver { phase, gust: 1.0 }.evaluate(assembly, 1.3);
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
    let instances = showcase_instances(&library, &mut rng);

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

            let results: Vec<RayResult> = gpu.run_variant(
                "scene_rays",
                Variant::for_slabs(topology.len()),
                cases.len() as u32,
                &[
                    (0, {
                        // the automatic layout omits attributes and materials, which tracing never reads
                        let mut bindings = topology_bindings(&topology);
                        bindings.push((3, template_bvh.as_entire_binding()));
                        bindings.push((4, meta_buffer.as_entire_binding()));
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
                ],
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
                "  {mode:?} {hierarchy:?}: {} rays, {cpu_hits} CPU hits, {} disagreements",
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
            showcase_instances(&library, &mut Rng(9)),
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

            for (hierarchy, variant) in [
                (HierarchyKind::Tree64, Variant::Default),
                (HierarchyKind::Tree64, Variant::Stackless),
                (HierarchyKind::Octree, Variant::Default),
            ] {
                let scene = GpuSceneData::pack(&snapshot, &library, &geometry, hierarchy);
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
                    gpu.run_variant("scene_rays", variant, cases.len() as u32, &groups, &output)
                };
                // warm up (pipeline creation, caches), then time
                run();
                let start = std::time::Instant::now();
                let runs = 5;
                for _ in 0..runs {
                    run();
                }
                let ms = start.elapsed().as_secs_f64() * 1000.0 / runs as f64;
                eprintln!(
                    "      {hierarchy:?} ({variant:?}): {width}x{height} primary rays in {ms:.1} ms ({:.1} Mrays/s, includes readback)",
                    cases.len() as f64 / ms / 1000.0
                );
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
            eprintln!("    {mode:?} Tree64: {ms:.1} ms (includes readback)");
        }
    }
}
