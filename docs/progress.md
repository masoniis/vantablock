# Progress

Where the engine stands against the [design spec](./design.md): what each phase delivered, what
was measured and decided, what comes next, and known issues. [`architecture.md`](./architecture.md)
maps the code itself; this file is the one that changes as work lands.

## Status

Phase 1 (renderer feasibility and memory contracts) is complete on Metal and Vulkan; Phase 2 is
under way, starting with footprint LOD filtering, which has a CPU reference, a matching shader
variant for single modules and whole scenes, and an opt-in primary visibility path
(`RenderSettings::footprint_lod`, off by default: at a one-pixel cone it filters almost nothing,
and a four-pixel cone saves up to 37% of primary visibility time but awaits the comparison
against supersampled references; see [Phase 2 progress](#phase-2-progress)).

The gate passed on Metal (M1 Pro) and Vulkan (RTX 3080). The 64-tree, short private traversal stack, slab-address residency, and visibility packing were kept on measured evidence on both backends; flattened pose groups replaced the template mini-BVH as the runtime default. The optional Vulkan hardware broad phase (ray queries over a TLAS of pose-frame boxes) matches the reference and is the fastest path measured, taking 25 to 42% less GPU time than the software BVH4; it is wired into the renderer as the default on adapters with ray queries (TLAS rebuilt every frame, primary visibility and shadow rays both using it, 34% less frame GPU time in game at 1280x720), with the software BVH4 as the always-available fallback. Driver register and spill statistics were captured on NVIDIA (128 registers per thread for the stack kernels, 80 stackless or with ray queries).

## Known issues

- **Metal, software broad phase: the GPU hangs in game.** On an M1 Pro a submission eventually
  never completes (wgpu: "timed out while waiting on the last successful submission"), the
  window goes black, and the server drops the client. With flattened groups (the default) it
  takes about ten seconds of singleplayer; with the template mini-BVH it survived 45 seconds but
  hung later while paused. `gpu_scene_traversal_matches_cpu` passes for every organization on
  the same adapter, so the trigger is likely a scene state its rays don't reach (a pose, the day
  cycle, or a camera ray). Found October 19, 2026, and reproduced on `d71582dc` with the
  hardware broad phase switched off, so it predates the UI overhaul.
- **No hardware broad phase on Metal.** naga's MSL backend (wgpu 29) lowers `rayQueryProceed` to a
  flag only `rayQueryTerminate` clears, over a single closest-hit `intersect`, so the broad
  phase's candidate loop never ends. `render::device::ray_query_supported` keeps Metal on the
  software BVH4; revisit when naga steps intersection queries with `next()`.

## Phase 1 results

Phase 1 is complete on Metal and Vulkan: every deliverable of the spec's renderer feasibility
phase, including the optional hardware broad phase, is built and held to the reference, each
comparison it asks for has been measured on both backends, and each decision is in the renderer:
the 64-tree, flattened groups, and, on adapters with ray queries, the hardware broad phase are
the runtime defaults.

- **Metal:** an M1 Pro. GPU times are 1280x720 primary rays including readback.
- **Vulkan:** an RTX 3080 (NVIDIA 616.92, wgpu 29). GPU times are the kernel alone, from
  timestamp queries; the readback adds a near-constant 5 ms there, which hid real differences.
- Both come from `gpu_scene_throughput` and `gpu_multi_slab_cost` (`--release`), CPU times from
  `just bench`, and register counts from `just shader-stats`.

**Gate.** Both hierarchies produce the reference's hits with no missed geometry, rotated pose
groups stay hittable, and visibility reconstructs exact terminal-cell identity, world position,
and motion, including across deduplicated geometry and slab boundaries. Template-shared mini-BVHs
are correct for every legal pose and fall back safely outside their envelopes. The whole
conformance suite passes on both adapters with zero disagreements, and so does the hardware
broad phase.

### Local hierarchy: 64-tree vs. 8-way octree with `8^3` bricks

| Asset                      | 64-tree topology / attributes | Octree topology / attributes |
| -------------------------- | ----------------------------- | ---------------------------- |
| oak (8 modules)            | 331 KiB / 1558 KiB            | 324 KiB / 1769 KiB           |
| meadow tile (1024^3 grass) | 1403 KiB / 1144 KiB           | 1018 KiB / 5244 KiB          |
| crystal grotto (6 modules) | 1339 KiB / 2356 KiB           | 1195 KiB / 5961 KiB          |

The octree's topology is a little smaller, but its 512-voxel attribute blocks deduplicate far less
than the 64-tree's 64-voxel ones, so it needs up to 4.6x the attribute memory on repetitive
foliage. On the CPU reference the octree traces 1.25 to 1.75x faster; on the GPU it is slower in
every scene and organization, by 26 to 41% on Metal and 29 to 64% on Vulkan (e.g. 10.8 vs. 7.2
ms on the grove, flattened).

**Decision:** the 64-tree stays the default; the octree remains selectable at runtime (`H`/`F6`)
and in every test. The sparse brick grid was not built: the spec needs one comparison, and the
octree already isolates the effect of fan-out and brick size.

### Pose-group organization: template mini-BVH vs. refit vs. flattened

| Scene, 64-tree               | Template mini-BVH | Instance refit | Flattened | Flattened, hardware |
| ---------------------------- | ----------------- | -------------- | --------- | ------------------- |
| Showcase, Metal              | 149 ms            | 106 ms         | 111 ms    | n/a                 |
| Grove, Metal                 | 231 ms            | 120 ms         | 109 ms    | n/a                 |
| Showcase, Vulkan (86 inst.)  | 7.95 ms           | 6.07 ms        | 7.62 ms   | 4.42 ms             |
| Grove, Vulkan (300 animated) | 12.60 ms          | 7.08 ms        | 7.17 ms   | 5.30 ms             |

On the grove, the template's shared mini-BVHs take 1.4 KiB but its rays visit 28.9 scene nodes, 69
mini-BVH nodes, and 7.8 candidate groups each, because instances are bounded by sphere-swept
motion envelopes. Refitting costs 138 KiB of per-instance nodes and visits 12.8 scene nodes, 15.6
mini-BVH nodes, and 6.5 groups; flattening costs a 152 KiB scene BVH and visits 21.2 nodes and 6.6
groups. Every snapshot builds in under 1 ms. Refit and flattened trade places by scene and
backend, within 26% of each other; the template mini-BVH loses everywhere.

**Decision:** flattened groups stay the runtime default (`B`/`F4` cycles all three): they are
never far behind refit and are the organization the hardware broad phase accelerates. Template
mini-BVHs are correct but lose on loose envelopes; tighter envelopes (sweeping only a hinge's legal
arc rather than a sphere) and thousands of foliage groups are Phase 3 questions, where this
comparison should be rerun.

### Hardware broad phase: Vulkan ray queries vs. software BVH4

The optional hardware path replaces the flattened scene BVH4 with a TLAS: one shared BLAS of a
unit box (wgpu 29 builds triangle geometry only, not procedural AABBs), instanced once per pose
frame onto its group-local bounds (padded by 1/64 voxel), with the frame index as custom data.
Box faces are non-opaque, so the query returns every face crossing as a candidate; the shader
traces a frame's 64-tree at its first crossing (the entry face, or the exit face when the ray
starts inside the box) and keeps the nearest hit itself. The boxes are oriented with their pose
frames, so they are tighter than the software BVH's world-space bounds.

It finds exactly the CPU reference's hits for both hierarchies, and it is the fastest path
measured: it takes 42% less time than the software flattened path on the showcase and 26% less on
the grove (25 to 27% less than refit). The TLAS of the grove's 1900 frames builds in 0.9 ms. Its kernel also needs
80 registers per thread instead of 128 (below).

**Decision:** make it the renderer's default broad phase where `GpuCapabilities::ray_query` holds,
never a requirement (Metal has no ray queries in wgpu). `RenderSettings::hardware_broad_phase`
(on by default, `G`/`F7`) selects it for flattened groups on single-slab geometry; every other
case, and every adapter without ray queries, traces the software BVH4.

In the renderer, prepare writes one TLAS instance per pose frame from the snapshot's frame boxes
(`client::render::gpu::broad_phase`), and a `voxel/scene_tlas` span builds the TLAS in the camera
schedule ahead of the passes, so its cost is part of the measured frame. Primary visibility and
direct lighting's shadow rays both compile in the `ray_query` variant. In game on the showcase
(229 objects, 585 pose frames; 1280x720 on the RTX 3080; `VANTA_GPU_TIMINGS=1`):

| Render scale   | Broad phase | TLAS build | Primary visibility | Direct lighting | Total   |
| -------------- | ----------- | ---------- | ------------------ | --------------- | ------- |
| 1.0 (1280x720) | software    |            | 4.85 ms            | 2.76 ms         | 7.61 ms |
| 1.0 (1280x720) | hardware    | 0.14 ms    | 2.99 ms            | 1.89 ms         | 5.02 ms |
| 0.5 (default)  | software    |            | 1.79 ms            | 1.88 ms         | 3.67 ms |
| 0.5 (default)  | hardware    | 0.14 ms    | 1.55 ms            | 1.28 ms         | 2.97 ms |

The per-frame TLAS build stays a small fixed cost, and the hardware frame takes 34% less GPU time
at full resolution (19% at the default half scale), consistent with the benchmark. Shadow rays
gain as much as primary rays. The two paths render identically.

### Residency: single slab vs. multi-slab

Pages are packed whole into at most four slabs per arena. On Metal, reading through a slab switch
at every node fetch cost the DDA loop about 10% even when everything was in one slab, so every
shader compiles a single-slab variant (direct indexing) and a multi-slab one, chosen from the
uploaded library. Spread over two topology and four attribute slabs, the grove traces 11%
(template) to 19% (flattened) slower on Metal. On Vulkan the difference is within noise (+3% for
template, -6% for flattened), and the multi-slab kernel uses the same 128 registers.

**Decision:** keep the `8 + 24` handle format and slab-address page tables; the multi-slab path is
correct on both backends and its cost is known. Keep the single-slab fast path for Metal.

### Traversal state: short private stack vs. stackless restart

The stackless variant keeps no parent state and re-walks the packed path from the root on each
pop. It is bit-identical to the stack. On Metal it is 12 to 29% slower in every scene and
organization. On Vulkan it ranges from 2% slower to 9% faster. The driver's statistics explain
why:

| Kernel (NVIDIA, `just shader-stats`) | Registers | Local memory (bytes, inferred) |
| ------------------------------------ | --------- | ------------------------------ |
| `scene_rays`, short stack            | 128       | 496                            |
| `scene_rays`, stackless              | 80        | 320                            |
| `scene_rays`, multi slab             | 128       | 496                            |
| `scene_rays`, hardware broad phase   | 80        | 128                            |
| `primary_visibility`                 | 128       | 496                            |

At 128 registers an Ampere SM holds 16 warps; at 80 it holds 24. The stackless variant buys that
occupancy back by re-walking nodes, which roughly breaks even. The private stacks live in local
memory, not registers: the reported local memory drops by the 176-byte 64-tree stack in the
stackless kernel, and by the scene and mini-BVH stacks as well with ray queries. The driver
reports local memory with bit 36 set, so these values are the low bits, an inference that the
stack sizes corroborate.

**Decision:** keep the short private stack: it is the faster choice on Metal and on par on
Vulkan. Register pressure (128) is the limiter on NVIDIA, so register-saving changes such as
narrower stack entries or splitting scene search from voxel traversal should be measured with
`just shader-stats` before and after.

### Cross-backend notes

- **DX12** cannot compile the traversal kernels with wgpu's default FXC compiler ("forced to
  unroll loop, but unrolling failed"); it would need DXC. Vulkan is the Windows target, and wgpu
  and Bevy pick it ahead of DX12 when both are present.
- Metal and Vulkan agree on every conformance case of the software paths; no backend-specific
  shader path was needed. The hardware broad phase is Vulkan only (see Known issues).

## Phase 2 progress

### Footprint LOD filtering

**Done (October 20, 2026): the CPU reference.** `voxel::lod::FootprintFilter` describes a ray
cone (`radius + spread * t` in world units, scaled into module voxels by the ray's direction
length) and a threshold. `tree64::trace_filtered` shares the 64-tree DDA loop with `trace`: when
it would descend into an occupied node whose cell edge fits inside the footprint's diameter and
that node has a summary, it uses the spec's axis-coverage proxy
(`clamp(sum(|d_axis| * coverage_axis), 0, 1)`, `LodSummary::opacity`) instead. The ray hits the
node's box when the threshold is below that opacity and passes through it otherwise. Filtered hits
report the aligned corner as `cell`, the level as the new `VoxelHit::lod` (`0` is an exact voxel),
and the summary's material, with `ArenaHandle::INVALID` as the attribute. `GeometrySource` gained
`lod_record`, and `GpuGeometryLibrary` keeps the LOD records (`lod`).
Tests: unit cases in `tree64` (wide vs. narrow footprints, pass-through below the threshold,
distance and ray-scale growth) and `footprint_filtering_reports_aligned_occupied_regions` in
`tests/hierarchy_reference.rs` (a zero cone matches `trace` exactly; wide-cone hits are aligned,
occupied, and at the box entry).

**Done (October 20, 2026): the shader mirror.** The LOD records are uploaded as one storage
binding (group 0, binding 11, `lod_records` in `lib/geometry.wesl`; the renderer's geometry layout
now needs 15 storage buffers per stage, and the library build fails if the records outgrow one
binding). `lib/lod.wesl` mirrors `voxel::lod` (`FootprintFilter`, `footprint_covers`, and
`lod_opacity` unpacking the f16 coverage with `unpack2x16float`), and `trace_tree64_filtered` in
`lib/tree64.wesl` mirrors `trace_filtered`: `trace_tree64` is now the filtered traversal with a
zero cone. The filter test lives in `footprint_decision`, which only the new `footprint_lod` WESL
feature compiles in (it reads the child's `lod_meta` word only when the footprint covers the
cell); without the feature it always says "descend", so the production kernels are unchanged
(`just shader-stats`: still 128 registers for the software path, 80 with ray queries). `VoxelHit`
gained `lod`, and a filtered hit carries its LOD record index in `attribute_handle` so shading can
read the summary's normal and material. `build.rs` builds `voxel_conformance_footprint_lod` in
single- and multi-slab forms; no pass uses the feature yet.

Tests: `gpu_footprint_filtering_matches_cpu` traces 18,432 rays (1,024 per real-asset module,
axis-aligned and random, with direction scales of 0.5, 1, and 3 as pose frames produce) per cone
for four cones (zero, pixel-like `0.25 + 0.01 t` at `0.5`, wide `2.1 + 0.05 t` at `0.1`, and
`1.0 + 0.02 t` at `0.37`) in both residency layouts, and requires the CPU's exact result: hit or
miss, cell, level, material (the voxel's or the summary's), `t` to `1e-4`, and DDA step count. All
match on the RTX 3080 (filtered hits per cone: 0, 7,530, 13,402, 9,577). Footprints that exactly
fit a cell are ties `length(d)` can round either way: a radius of exactly `2` disagreed on one
ray at `t = 0`, so the test avoids exact fits. Driver statistics: the bare 64-tree `rays` kernel
(exact, which also contains the octree) uses 64 registers, `filtered_rays` 80.

**Done (October 20, 2026): filtered scene traversal.** One world-space cone now reaches every
frame: `trace_scene_filtered` in `lib/scene.wesl` threads a `FootprintFilter` through
`trace_frame`, `try_frame`, `try_bounded_frame`, and `intersect_instance` in both broad phases
(`trace_scene` is the filtered trace with `exact_footprint()`, so the passes are unchanged). Local
rays are never renormalized, so a frame's direction length is its voxels per world unit and the
cone needs no rescaling, on the GPU as on the CPU. The CPU mirror is
`VoxelRoot::trace_filtered` (`tree64::trace_filtered` for 64-tree roots, exact for octrees) as
the frame trace of `SceneSnapshot::intersect`. `build.rs` adds
`voxel_conformance_footprint_lod_ray_query`.

The scene test found a real bug: a filtered node's aligned box can stick out of its module's
occupied bounds, which every broad phase culls frames with (scene BVH leaves, mini-BVH envelopes,
per-frame bounds, and TLAS boxes), so a ray grazing the protruding part hit the summary in an
exhaustive search but not through the BVHs (the CPU's `InstanceRefit` disagreed with brute force).
Filtered hits are now where the ray enters the node's box clipped to the root's occupied bounds,
and a ray that misses the clipped box passes through. That needed the bounds on the GPU:
`PoseFrame` grew `local_bounds_min`/`local_bounds_max` (192 to 224 bytes, `layout::root_bounds`),
`VoxelRoot` in `dda.wesl` carries them, and `entry_face` takes a box's two corners on both sides.

Tests: `filtered_scene_traces_match_exhaustive_search_in_world_units` (voxel: every organization
and hierarchy matches an exhaustive filtered search, a zero cone equals `intersect`, the octree is
never filtered, and each filtered cell's world edge fits the cone's world diameter),
`filtered_boxes_are_clipped_to_the_occupied_bounds` (tree64 unit), and
`gpu_filtered_scene_traversal_matches_cpu`: 4,304 showcase rays per cone for four world-space
cones (zero, a 1280x720 pixel's `spread = 0.0008`, `0.02 + 0.004 t` at `0.1`, and
`0.01 + 0.002 t` at `0.37`) for every organization and hierarchy, both residency layouts, and
the ray query variant, requiring the CPU's frame, cell, level, and `t` (to `1e-3`). All agree on
the RTX 3080 with zero disagreements (filtered hits per cone: 0, 1, 2,898, 414; the pixel cone
filters almost nothing within the showcase's 26 m). The module-level filtered counts moved
slightly with clipping (7,510, 13,142, 9,511).

Measured (`gpu_scene_throughput --release`, RTX 3080, two runs, `filtered_scene_rays` vs.
`scene_rays`; the filtered kernel also writes 32-byte results instead of 16): with the hardware
broad phase the zero-cone variant is within noise (showcase 4.50 to 4.69 ms vs. 4.69 to 4.67 ms;
grove 5.43 to 5.68 ms vs. 5.51 to 5.55 ms); on the software BVH4 it costs 0 to 11% (showcase
flattened 7.66 to 7.73 ms vs. 7.94 to 8.07 ms; grove refit 6.84 to 6.95 ms vs. 7.59 to 7.62 ms).
The pixel cone costs the same as the zero cone. `just shader-stats`: production kernels unchanged
(128 registers software, 80 ray query); `filtered_scene_rays` uses the same (128 and 80).

**Open decisions:** a fixed `0.5` threshold makes sparse foliage disappear at distance (the
reference test needs `0.1` to see many filtered hits), so the threshold should probably become a
stable per-pixel random value (stochastic transmission). The octree has no summaries and is never
filtered, and the footprint rule (cell edge vs. cone diameter) is untuned. At one pixel's cone the
showcase barely filters at all, so filtering's payoff is at distance or at reduced render scale
(the cone should use the traced grid's pixel size, not the target's).

Compile-time variant or runtime uniform: on the default (ray query) path the variant costs nothing
measurable with a zero cone, and 0 to 11% on the software fallback, with no register change. That
argues for a runtime toggle that switches between the exact and `footprint_lod` pipelines (the
variant compiled for every production kernel) rather than a uniform in the exact kernels; step 1
decides it when wiring the passes.

**Done (October 20, 2026): filtered primary visibility.** Primary visibility traces each pixel's
ray cone through `trace_scene_filtered` in `footprint_lod` builds (`pixel_footprint`: no radius,
and a spread of half the angle between the traced pixel's left and right edges, times
`footprint_scale`, so reduced render scales filter more; exact builds pass `exact_footprint()`
and are unchanged). `vis.x` now carries the hit's level, and `vis.y` the LOD record index for
filtered hits. `build.rs` builds `primary_visibility_footprint_lod` in single-slab, multi-slab, and
ray query forms, and `PrimaryVisibilityPipeline` holds an exact and a footprint LOD set of
variants, chosen per frame by the new `RenderSettings::footprint_lod` (F8 or L, listed in the
pause menu and shown in the debug overlay) with `footprint_threshold` (`0.5`) and
`footprint_scale` (`1` pixel) reaching the shader through two new `ViewUniform` fields (224 to 240
bytes). The decision: a runtime switch between pipelines, not a uniform in the exact kernels.
Direct lighting needs no variant: every build reads the level, and filtered hits shade with the
summary's material and mean normal (`lod_normal` in `lib/lod.wesl` decodes the octahedral
normal as `LodSummary::unpack` does; the hit face's normal where the summary's faces cancel),
while shadow rays stay exact and leave from the hit face. A `LodLevels` debug view shows exact
voxels grey and filtered hits from blue (level 1) to red.

Tests: `gpu_footprint_filtering_matches_cpu` now also requires the GPU's decoded summary normal
of every filtered hit to match `LodSummary::unpack` to `1e-5` (all match on the RTX 3080), and
`shaders_validate` covers the three new variants. `just test`, `just clippy`, and `just test-gpu`
pass. `just shader-stats`: the production kernels are unchanged (`primary_visibility` 128
registers software, 80 ray query); the footprint LOD forms use 128 (software) and 96 (ray
query, vs. 80 for `filtered_scene_rays`; the pixel cone and the 32-byte hit are live longer).
BRP (release, 1280x720, render scale 0.5, showcase meadow): with a one-pixel cone, `LodLevels`
shows level 1 hits on the distant tree canopies and trunks only, and the shaded frame is
indistinguishable from the exact one at a glance; at `footprint_scale = 4` the grass, canopies,
and rotor blades turn visibly blockier. In-game FPS stayed at the 144 Hz cap with filtering on
and off, at render scales 0.5 and 1.0, so it says nothing about cost; the kernel cost is the
`filtered_scene_rays` measurement above.

**Open decisions:** filtered summaries are opaque boxes at a fixed threshold, so a summary normal
can face away from the camera (thin foliage seen from behind) and shadow rays from a filtered
box can self-shadow on the node's own voxels; both await the reference comparison. Whether
filtering should become the default depends on that comparison and on an in-game GPU time
measurement (`VANTA_GPU_TIMINGS=1`, or a `primary_visibility` kernel benchmark), since the frame
rate is capped.

**Done (October 20, 2026): the cost and payoff of filtered primary visibility.** The new
ignored benchmark `gpu_primary_visibility_cost` (`gpu_conformance.rs`; run with
`cargo test --release -p vantablock-client --test gpu_conformance -- --ignored --nocapture gpu_primary_visibility_cost`)
dispatches the production `primary_visibility` and `primary_visibility_footprint_lod` kernels
(both broad phases, flattened groups, 64-tree, single slab) into real visibility, depth, cell, and
stats textures with a `GpuViewUniform` built like the game's (60 degree vertical field of view,
infinite reverse-z, 1280x720 target), at render scales 0.5 and 1.0, exact and with one- and
four-pixel cones (`footprint_scale`, threshold `0.5`). It reports kernel time (timestamp queries,
mean of 50 dispatches after 5 warm-ups), mean voxel DDA steps per pixel from `stats`, and the
share of hits that are filtered from `vis`. Views: the throughput benchmark's showcase street
view, a grass-level diagonal across the meadow, an elevated overview, and the 300-instance grove
from 48 m. The conformance harness gained `Gpu::bind_module` and `Gpu::time_dispatch` (any
module, any 3D dispatch).

Measured (RTX 3080, `--release`, two runs; dispatch-to-dispatch noise is about 8%). Kernel time
as the mean of the runs, hardware broad phase / software BVH4:

| View, grid | Exact (ms) | 1 px cone | 4 px cone | DDA steps/px exact, 4 px | Filtered hits 1 px, 4 px |
| --- | --- | --- | --- | --- | --- |
| street, 640x360 | 1.97 / 3.08 | +5% / +2% | -37% / -33% | 39.6, 36.2 | 0.3%, 44% |
| street, 1280x720 | 3.77 / 6.15 | +5% / -5% | -3% / -7% | 39.6, 39.1 | 0%, 9% |
| grass level, 640x360 | 1.05 / 1.14 | +1% / +7% | -4% / +3% | 39.0, 37.1 | 0%, 27% |
| grass level, 1280x720 | 2.39 / 2.74 | +7% / +7% | +3% / -1% | 39.0, 38.5 | 0%, 7% |
| overview, 640x360 | 1.31 / 1.48 | +3% / -1% | -22% / -26% | 32.4, 20.3 | 8%, 100% |
| overview, 1280x720 | 2.30 / 3.63 | +12% / +5% | -22% / -19% | 32.5, 23.0 | 0.2%, 82% |
| grove, 640x360 | 1.50 / 1.95 | +2% / -7% | -18% / -22% | 21.6, 17.3 | 3%, 81% |
| grove, 1280x720 | 4.75 / 5.24 | 0% / +2% | 0% / -13% | 21.6, 20.5 | 0.1%, 23% |

What it says: at the default one-pixel cone, filtering buys nothing in these views (at most 8% of
hits filtered at half scale, 0.3% at full scale, and DDA steps unchanged) and costs 0 to 12%,
mostly within noise, so `footprint_lod` stays off by default. A four-pixel cone saves 18 to 37%
at half scale wherever the view reaches distance (street, overview, grove) and about 20% for the
overview at full scale, but nothing at grass level, where the expensive pixels are close. Time
falls faster than mean DDA steps (street at half scale: 9% fewer steps, 37% less time), so the
savings come from the long-tail pixels that finish early, not the average. Filtering grows hit
coverage (street at half scale: 90.6% to 92.2% of pixels), since a summary box is larger than the
voxels it covers: the silhouette dilation the reference comparison must judge.

**Next steps, one per session:**

1. Render supersampled reference frames (exact traversal, many rays per pixel, offline or as a
   test harness, reusing `gpu_primary_visibility_cost`'s view and texture setup) for the
   benchmark's views, compare the one-ray filtered frames against them (silhouette coverage,
   mean color error) at `footprint_scale` 1, 2, and 4, for deterministic vs. stochastic (stable
   per-pixel random) thresholds, and pick the threshold rule, the default cone width, and whether
   `footprint_lod` becomes the default. The cost table above is the other half of that decision:
   one pixel filters almost nothing, and four pixels is where the time savings are.
2. Bake SGGX-style directional summaries next to the axis-coverage proxy from the same voxels
   and run the same comparison (spec's "Additional Phase 2 test"): angular opacity, silhouettes,
   LOD bytes, and frame time; keep the axis proxy if it meets the quality target.

## What comes next

Performance is the open problem for Phase 2: the showcase traces in about 110 ms at 1280x720 on
an M1 Pro (software broad phase) and 4.4 ms on an RTX 3080 (hardware broad phase), and grass and
foliage take about 170 DDA steps per pixel, which no broad phase removes. The levers are the
spec's: footprint LOD (summaries are baked but unused) and SGGX directional summaries, then
temporal filtering, which can now build on reconstructed motion. Register pressure (128 per thread
on NVIDIA for the software path, 80 with ray queries) is worth watching with `just shader-stats`
as traversal grows.

The roadmap, in the spec's order:

- Phase 2: footprint LOD filtering, foliage filtering candidates, anti-aliasing, atmosphere, and
  an occlusion term for sky light (caves are as bright as the open meadow). The reference GPU is
  the RTX 3080 (Vulkan, hardware broad phase), with the M1 Pro (Metal, software broad phase) as
  the lower tier.
- Phase 3: wind-driven deformation (Tier 2), tighter motion envelopes, thousands of articulated
  groups (rerunning the organization comparison with the hardware broad phase, and TLAS rebuild
  vs. refit), and animation LOD.
- Phase 4: GI, temporal filtering, homogeneous media, transparent and refractive crystals;
  secondary rays can share the TLAS on capable adapters.
- Phases 5 to 7: simulation, copy-on-write editing, streaming, and a camera-relative world origin
  for very large worlds.
