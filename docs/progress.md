# Progress

Where the engine stands against the [design spec](./design.md): what each phase delivered, what
was measured and decided, what comes next, and known issues. [`architecture.md`](./architecture.md)
maps the code itself; this file is the one that changes as work lands.

## Status

Phase 1 (renderer feasibility and memory contracts) is complete: its gate passed on Metal (M1 Pro)
and Vulkan (RTX 3080), and the 64-tree, flattened pose groups, and, on adapters with ray queries,
the hardware broad phase are the runtime defaults (see [Phase 1 results](#phase-1-results)).

Phase 2 is under way. Footprint LOD filtering is closed out as an opt-in path
(`RenderSettings::footprint_lod`, plus `footprint_transmission`; see
[Phase 2 results](#phase-2-results)): LOD records hold an SGGX tensor fitted at bake time to
measured coverage, since the spec's `L1` axis proxy overestimates it everywhere. Opaque filtered
hits save up to 39% of primary visibility time but leave filtered pixels worse than exact in most
views; transmission meets that target with a two-pixel cone at half scale but costs more than
exact traversal, so both stay off by default. Temporal anti-aliasing (`RenderSettings::temporal_aa`)
is on by default: jittered primary rays and a reprojecting, neighborhood-clipped resolve leave 30
to 55% of the one-ray frame's error against supersampled references in still views and cut it by
19 to 56% in a slow pan, for 0.06 ms at 1280x720. Its second slice (history validity, temporal
upsampling) is next.

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

## Phase 2 results

Measured on the RTX 3080 (Vulkan, `--release`), the Phase 2 reference GPU.

### Footprint LOD filtering

**Done, opt-in.** Every primary ray carries a pixel cone; where a 64-tree node's cell fits inside
the cone's footprint, the ray hits or passes the node's baked summary instead of descending.
`RenderSettings::footprint_lod` (off by default, `F8`/`L`, pause menu and debug overlay) enables
it, and `RenderSettings::footprint_transmission` (off, needs `footprint_lod`) further lets
filtered hits partially transmit. Both are held to a CPU reference on every variant.

#### Design

- **Filter (CPU reference):** `voxel::lod::FootprintFilter` is a cone (`radius + spread * t` in
  world units; local rays are never renormalized, so a frame's direction length scales it into
  voxels) and a threshold. `tree64::trace_filtered` shares `trace`'s DDA loop: an occupied node
  whose cell edge fits the footprint's diameter is tested with the record's opacity and stops the
  ray when the threshold is below it. Filtered hits report the aligned corner as `cell`, the level
  as `VoxelHit::lod` (`0` is exact), and the summary's material. `VoxelRoot::trace_filtered` is
  the frame trace of `SceneSnapshot::intersect`; octrees have no summaries and are never filtered.
- **Occupied boxes:** a filtered hit fills its node's occupied box, clipped to the root's occupied
  bounds, and a ray that misses the box passes through. The clip is required: every broad phase
  culls frames by those bounds (`PoseFrame::local_bounds_min`/`max`, 224 bytes), so an unclipped
  box sticking out of them was hit by exhaustive search but not through the BVHs.
- **LOD record (16 bytes):** words 0 and 1 hold the fitted SGGX tensor's axis coverages
  `c_a = sqrt(S_aa)` (f16) and the material; word 2 its correlations `r_ab = S_ab / (c_a c_b)`
  (three 10-bit offset snorms, `lod::stored_tensor`); word 3 the occupied box (4 bits per bound in
  sixteenths of the node edge, rounded outward) and the solid volume fraction (unorm8). The
  material is the one with the most exposed faces (a face is exposed only when the neighbor voxel
  in the module is empty). Coverage is relative to the occupied box, a departure from the spec's
  node-relative coverage: opacity is the chance of a hit given that the ray enters the box, and
  node-relative coverage (box value times the box face's share, `lod_node_coverage`) weights
  shading. `.vanta` files hold canonical modules, so the record format needs no `BAKER_VERSION`
  bump.
- **Bake measurement:** `BakeSetup::tensor` (`bake_setup()` for `BAKE_MEASUREMENT`) measures 16
  directions and least-squares fits all six tensor terms to measured `coverage^2`. Bricks use
  `BrickRays`: per direction, the distinct voxel sets that 64x64 jittered lines through a brick
  cross (about 360), weighted by line count, so a brick's coverage is a weighted count of the sets
  meeting its mask over those meeting its occupied box (once per distinct occupancy). Larger nodes
  march 8x8 stratified rays per direction through a sparse brick-mask `NodeVoxels`. `tree64::bake`
  measures every distinct brick mask and internal node on all cores before building
  (`NodeTensors`).
- **Opacity:** `LodSummary::opacity` and `lod_opacity` (`lib/lod.wesl`) evaluate
  `min(sqrt(max(w^T S w, 0)), 1)`.
- **Primary visibility:** `pixel_footprint` uses no radius and a spread of half the traced pixel's
  angle times `footprint_scale` (`1`), so reduced render scales filter more; `footprint_threshold`
  is `0.5`. Both reach the shader through `ViewUniform`. `PrimaryVisibilityPipeline` holds exact,
  footprint LOD, and `footprint_transmission` variant sets and picks one per frame (a pipeline
  switch, not a uniform in the exact kernels). `RenderSettings::footprint_stochastic` (off) sets
  `VIEW_STOCHASTIC_FOOTPRINT`: each pixel's threshold comes from a stable PCG hash, so a summary of
  opacity `a` stops a fraction `a` of rays.
- **Transmission:** the front trace uses threshold `0`, so any summary the cone covers becomes the
  front record (`0` and `0.25` measured identically). `continuation` computes `a` and the exit of
  the clipped occupied box in the frame's local ray; if `a < transmission_opaque` (`0.9`,
  `ViewUniform`), a second `trace_scene_filtered` starts just past the exit (`1e-5` relative, so
  the same node can't be hit again) with the pixel's `0.5` threshold. Both traces share one call
  site in a two-iteration loop with only the packed front record live across it (`trace_pixel`).
- **Visibility buffer:** every target holds two records: `vis` `Rgba32Uint` (front, behind identity
  and handle; `vis.x` the front's level, `vis.y` its LOD record index), `depth` `Rgba32Float`
  (front t, behind t, coverage `a`, `1` for opaque and exact hits), `cell` `Rg32Uint`. Exact and
  opaque footprint LOD kernels write `a = 1` and no record behind.
- **Lighting:** `shade` (`direct_lighting.wesl`) shades both records with `shade_record` (each with
  its own shadow ray and fog) and mixes them by `a`; a missing record behind is the sky.
  `filtered_face_light` averages the sun cosine, sky height, and ground bounce of the node's three
  camera-facing faces, each weighted by `|d_a|` times its node-relative coverage (from the fit's
  `c_a`); its one shadow ray leaves the occupied box toward the light, so the node can't shadow
  itself. Debug views show the front record; `LodLevels` shows exact voxels grey and filtered
  levels blue to red.
- **Shader variants and registers:** the `footprint_lod` WESL feature (`lib/lod.wesl` mirrors
  `voxel::lod`; `trace_tree64_filtered` and `trace_scene_filtered` thread the filter through both
  broad phases, and the exact traces are the filtered ones with `exact_footprint()`) is the only
  one reading the LOD records (geometry binding 11, which needs 15 storage buffers per stage). The
  `transmission` feature adds three more primary visibility builds (single slab, multi slab, ray
  query). Registers (NVIDIA, software / ray query): exact 128 / 80, footprint LOD 128 / 96. It is a
  separate variant because any second trace site in the footprint LOD kernel raised it to 168 /
  128 (the driver moves the private stacks into registers) and cost the opaque path about ten
  points of its savings even with the second trace never taken.
- **Tests:** CPU (`tree64` units, `footprint_filtering_reports_aligned_occupied_regions`,
  `filtered_scene_traces_match_exhaustive_search_in_world_units`,
  `baked_tensors_match_the_directional_study`, `brick_line_sets_measure_what_rays_do`) and GPU
  conformance (`gpu_footprint_filtering_matches_cpu`, `gpu_filtered_scene_traversal_matches_cpu`:
  cell, level, material, `t`, and decoded correlations on every organization, hierarchy, residency,
  and ray query variant) agree with zero disagreements. Cones that exactly fit a cell are ties
  `length(d)` can round either way, so the tests avoid exact fits.
- **Benchmarks** (ignored tests in `gpu_conformance.rs`, `--release --nocapture`):
  `gpu_primary_visibility_cost` dispatches the production kernels (both broad phases) in four views
  (street, grass level, overview, the 300-instance grove from 48 m) at render scales 0.5 and 1.0,
  exact and filtered, and reports kernel time, DDA steps, the filtered share, and the share tracing
  a second record. `gpu_footprint_lod_quality` renders the same views through both production
  passes at one ray per pixel (exact; 1, 2, and 4 px cones; fixed and stochastic thresholds;
  opaque and transmission) against a 4x4 supersampled exact reference: per-pixel and 3x3-filtered
  color error, coverage bias, and the filtered pixels' error next to the exact frame's on the same
  pixels, with PNGs under the temp directory's `vanta-footprint-quality`. The directional study is
  `directional_lod_comparison` (`cargo test --release -p vantablock-baker --test directional_lod
  -- --ignored --nocapture`, 8 s).

#### Measurements

**Summary accuracy** (`directional_lod_comparison`): `voxel::angular` measures a node's angular
coverage with 48x48 stratified parallel rays per direction and scores summaries on 96 directions
disjoint from the 48 the fits see. Mean `|error|` and the share of node-direction pairs whose
hit/pass decision at `0.5` differs from the measurement, up to 160 nodes per asset and size. The
axis proxy is the spec's `clamp(sum(|w_a| c_a))` (`L1`), diagonal is `diag(c_a^2)` (the same
coverages in `L2`), fitted is a 48-direction PSD fit stored as six f16, quartic is 15 spherical
harmonic coefficients (bands 0, 2, 4), and baked is the shipped record as stored. Sampling error is
about 1% per direction; the baked tensor's bias stays within +-0.005.

| Nodes                  | Axis proxy (`L1`) | Diagonal (`L2`) | Fitted SGGX  | Quartic      | Baked SGGX   |
| ---------------------- | ----------------- | --------------- | ------------ | ------------ | ------------ |
| diagonal/tilted leaves | 0.442, 48.6%      | 0.259, 44.8%    | 0.057, 5.9%  | 0.023, 2.3%  | 0.071, 9.5%  |
| oak 4^3                | 0.156, 5.0%       | 0.058, 3.6%     | 0.015, 1.2%  | 0.007, 0.7%  | 0.016, 1.5%  |
| oak 16^3               | 0.289, 38.1%      | 0.104, 21.7%    | 0.025, 5.0%  | 0.012, 2.2%  | 0.036, 7.7%  |
| oak 64^3               | 0.266, 45.3%      | 0.090, 11.4%    | 0.022, 1.8%  | 0.011, 1.1%  | 0.034, 2.5%  |
| crystal grotto 4^3     | 0.120, 0.6%       | 0.045, 0.6%     | 0.011, 0.3%  | 0.005, 0.1%  | 0.012, 0.3%  |
| crystal grotto 16^3    | 0.274, 20.9%      | 0.122, 18.9%    | 0.031, 5.1%  | 0.017, 2.6%  | 0.040, 7.4%  |
| crystal grotto 64^3    | 0.349, 43.4%      | 0.161, 32.2%    | 0.040, 6.9%  | 0.023, 4.2%  | 0.051, 9.5%  |
| meadow tile 4^3        | 0.144, 14.1%      | 0.045, 4.3%     | 0.013, 1.8%  | 0.008, 1.1%  | 0.014, 1.9%  |
| meadow tile 16^3       | 0.212, 29.8%      | 0.075, 3.3%     | 0.019, 0.4%  | 0.011, 0.4%  | 0.031, 1.1%  |
| meadow tile 64^3       | 0.263, 0.0%       | 0.086, 0.0%     | 0.015, 0.0%  | 0.010, 0.0%  | 0.025, 0.0%  |

The axis proxy overestimates coverage everywhere (bias +0.12 to +0.35) and fails the spec's test;
the fitted tensor is unbiased and within 0.035 of the quartic, so the quartic's 30 bytes aren't
worth it. 12x12 rays for internal nodes gain about 0.006 for 15% more bake time; 12 directions
with 6x6 rays lose about 0.01.

**Frame quality** (`gpu_footprint_lod_quality`, hardware broad phase), display-space color error
against the 4x4 reference as `per pixel / 3x3 / coverage bias` in points. Reference coverage:
street 90.6%, grass 84.2%, overview 70.5%, grove 74.6%.

| View, grid         | Exact               | 2 px opaque         | 2 px transmission   | 4 px opaque         | 4 px transmission   |
| ------------------ | ------------------- | ------------------- | ------------------- | ------------------- | ------------------- |
| street, 640x360    | 3.70 / 1.61 / +0.01 | 3.76 / 1.70 / +0.38 | 3.67 / 1.64 / +0.27 | 4.20 / 2.22 / +0.93 | 3.89 / 2.00 / +0.66 |
| street, 1280x720   | 2.32 / 1.18 / +0.01 | 2.32 / 1.18 / +0.01 | 2.32 / 1.18 / +0.01 | 2.50 / 1.37 / +0.38 | 2.43 / 1.30 / +0.26 |
| grass, 640x360     | 2.13 / 1.12 / 0.00  | 2.17 / 1.19 / +0.28 | 2.10 / 1.15 / +0.21 | 2.46 / 1.52 / +0.60 | 2.31 / 1.40 / +0.45 |
| overview, 640x360  | 4.87 / 1.69 / -0.02 | 4.71 / 1.87 / +0.41 | 3.83 / 1.57 / +0.27 | 4.83 / 2.12 / +0.42 | 3.72 / 1.68 / +0.33 |
| overview, 1280x720 | 3.70 / 1.34 / 0.00  | 3.70 / 1.36 / +0.03 | 3.65 / 1.34 / +0.02 | 4.26 / 2.03 / +0.40 | 3.53 / 1.64 / +0.27 |
| grove, 640x360     | 3.08 / 1.33 / +0.02 | 3.15 / 1.42 / +0.31 | 3.05 / 1.37 / +0.24 | 3.85 / 2.18 / +1.09 | 3.44 / 1.89 / +0.83 |
| grove, 1280x720    | 2.00 / 0.97 / 0.00  | 2.01 / 0.98 / +0.03 | 2.01 / 0.97 / +0.02 | 2.22 / 1.18 / +0.29 | 2.14 / 1.12 / +0.21 |

**Filtered pixels** (same benchmark): error on just the filtered pixels vs. the exact frame's on
the same pixels, in percent. The target is filtered pixels no worse than exact.

| View, grid         | 4 px opaque     | 2 px transmission | 4 px transmission |
| ------------------ | --------------- | ----------------- | ----------------- |
| street, 640x360    | 5.76 vs. 4.50   | 5.48 vs. 5.86     | 5.01 vs. 4.53     |
| street, 1280x720   | 6.28 vs. 4.02   | 3.21 vs. 3.02     | 5.47 vs. 4.05     |
| grass, 640x360     | 5.25 vs. 3.75   | 4.80 vs. 5.22     | 4.58 vs. 3.78     |
| overview, 640x360  | 6.69 vs. 6.77   | 5.04 vs. 6.82     | 5.17 vs. 6.78     |
| overview, 1280x720 | 6.21 vs. 5.27   | 3.56 vs. 4.43     | 4.98 vs. 5.27     |
| grove, 640x360     | 5.39 vs. 4.16   | 4.19 vs. 4.38     | 4.75 vs. 4.17     |
| grove, 1280x720    | 4.33 vs. 3.07   | 3.38 vs. 3.19     | 3.90 vs. 3.08     |

At a one-pixel cone at most 8% of hits are filtered at half scale and the frame is within 0.04
points of exact. Stochastic thresholds move results by at most 0.2 points, since box-relative
opacities sit near 0 or 1.

**Cost** (`gpu_primary_visibility_cost`, hardware / software broad phase). Exact kernel time, and
change against it; the 4 px ranges span runs with each summary (`L1`, `L2`, fitted SGGX), which
agree within noise, and dispatch noise is about 8%, more at 1280x720 on the hardware path.

| View, grid         | Exact (ms)  | 4 px opaque               | 2 px transmission | 4 px transmission |
| ------------------ | ----------- | ------------------------- | ----------------- | ----------------- |
| street, 640x360    | 1.87 / 2.68 | -29 to -39% / -28 to -34% | +1% / +16%        | +25% / +19%       |
| street, 1280x720   | 3.41 / 5.33 | within noise              |                   |                   |
| grass, 640x360     | 0.98 / 1.07 | -5 to -10% / -8 to -14%   | +34% / +33%       | +45% / +52%       |
| overview, 640x360  | 1.29 / 1.51 | -13 to -22% / -24 to -25% | +83% / +44%       | +78% / +46%       |
| overview, 1280x720 | 2.30 / 3.51 | -16 to -21% / -19 to -22% |                   |                   |
| grove, 640x360     | 1.39 / 1.83 | -8 to -9% / -16 to -18%   | +44% / +35%       | +59% / +61%       |
| grove, 1280x720    | 4.35 / 4.75 | within noise              |                   |                   |

A one-pixel cone costs 0 to 12%, mostly noise. Opaque savings come from long-tail pixels (time
falls faster than mean DDA steps); on the grove the hardware path saves little because rays that
miss a small occupied box keep traversing. 5 to 9% of pixels trace a second record at 2 px (47% in
the overview), 15 to 59% at 4 px; the second trace restarts scene search from the camera and
diverges within warps. `transmission_opaque` of `0.995` or `0.75` instead of `0.9` changed quality
by +0.0 / -0.1 points and time by +10% / -20%.

**Bake** (16 threads), 64-tree bake time with the fitted tensor (the axis record's in parentheses)
and 16-byte LOD records after dedup:

| Asset           | Bake time       | LOD records |
| --------------- | --------------- | ----------- |
| crystal cluster | 52 ms (37 ms)   | 7,387       |
| crystal grotto  | 260 ms (213 ms) | 52,860      |
| kinetic rotor   | 14 ms (10 ms)   | 1,160       |
| meadow tile     | 175 ms (141 ms) | 34,154      |
| oak             | 72 ms (48 ms)   | 16,090      |

#### Dead ends

- **`L1` axis proxy (the spec's):** overestimates coverage by 0.12 to 0.35 and flips 21 to 45% of
  16^3 and 64^3 decisions.
- **`L2` axis proxy (diagonal tensor):** free, cuts that error 1.7 to 3.2x, still biased; in frames
  the fitted tensor beat it by only 0.01 to 0.21 points of coverage bias.
- **Face-built tensor:** diagonal for voxels (axis-aligned faces), so it can't tell a leaf along
  `x = y` from one along `x = -y`; fitting to measured coverage is what captures diagonal leaves
  and facets.
- **Fixing the diagonal to exact axis coverages (`Sggx::fit_with_axes`):** keeps nearly all the
  bias (oak 16^3 0.100 vs. 0.104), since the bias is in the diagonal; the bake fits all six terms.
- **Quartic (30 bytes):** within 0.035 of the fitted tensor, not worth the bytes.
- **Mean normal and spread in the record:** shading with the mean normal made filtered pixels
  worse (net orientation, not the faces the camera sees); dropping it also improved dedup.
- **Self-shadowing and the single material** were not why filtered crystals were dark (0.05 to 0.1
  and 0.01 to 0.04 points).
- **Node-relative opacity:** cut coverage bias (overview 4 px +0.26) but left higher color error
  than box-relative.
- **Second trace inside the footprint LOD kernel:** see the register note above.
- **Single-threaded DDA bake measurement:** took seconds per asset.

#### Decisions

- **`footprint_lod` is off by default, with a one-pixel cone:** at one pixel it changes nothing,
  and opaque cones large enough to save time leave filtered pixels 1.3 to 2.3 points worse than
  exact in the street, grass, and grove views (only the overview wins).
- **`footprint_transmission` ships off** (`transmission_opaque` `0.9`): it improves every filtered
  frame and cuts coverage bias by a quarter to a third, and at 2 px and half scale filtered pixels
  beat exact ones in every view (the target), but every transmission configuration costs more
  than exact traversal. The opaque decision is the fast path, transmission the accurate one, and
  neither is both.
- **Thresholds stay deterministic (`0.5`):** stochastic thresholds trade bias for noise that only
  an averaging or temporal resolve removes.

#### Open issues

- **Coverage bias:** filtered frames still dilate silhouettes (+0.3 to +1.1 points opaque at 4 px,
  +0.2 to +0.8 with transmission), a low-frequency error the 3x3 filter doesn't remove. The better
  estimate flips few decisions, so what remains is what one summary per node can't express (a node
  partly covering the cone).
- **Transmission cost:** the lever, if revisited, is tracing the records behind in a second,
  compacted dispatch (only transmitting pixels, resuming from the front frame instead of the scene
  root) rather than in the primary kernel.
- **Grove hardware-path cost:** small occupied boxes leave the hardware broad phase saving -5 to
  +1% on the grove, against -18 to -28% on the software BVH4.
- **Untuned cone rule** (cell edge vs. cone diameter), and the 4x4 reference is itself limited:
  the exact one-ray frame is 2 to 5% from it, mostly grass and foliage aliasing.

### Anti-aliasing

**First slice done, on by default.** `RenderSettings::temporal_aa` (on, `F9`/`T`, pause menu and
debug overlay) jitters every primary ray within its pixel and blends each frame with the
reprojected history (the spec's `TemporalFilterNode`); the one-ray frame stays one setting away.

#### Design

- **Why temporal, not spatial:** a filter over a one-ray visibility buffer can only blur; it
  can't recover subpixel coverage. The 3x3 rows of `gpu_footprint_lod_quality` bound what one
  does, and the temporal resolve beats them (below). Jittered frames that converge to the pixel's
  box average are what the 4x4 reference computes.
- **Jitter:** `gpu::temporal::jitter` is the Halton (2, 3) sequence over 16 frames in
  `[-0.5, 0.5)` traced pixels, in `ViewUniform::jitter` (zero with the resolve off). Primary
  visibility traces `pixel + 0.5 + jitter` (the footprint cone and stochastic hash follow it), and
  lighting shades the same ray (`pixel_ray`). The projection matrices stay unjittered.
- **Reprojection:** with the resolve on, lighting renders the traced grid itself (`fs_temporal`:
  `Rgba16Float` color, tonemapped as before, and an `Rg16Float` offset) instead of the view target.
  `history_offset` maps the front record's hit through its pose frame's previous transform
  (`previous_world_position`) and `prev_clip_from_world` (a miss as a direction, `w = 0`), less this
  frame's jitter, since history holds unjittered pixel centers; points behind last frame's camera
  get an offset no history holds.
- **Resolve (`temporal_resolve.wesl`, compute):** samples the history at the offset through a
  five-tap Catmull-Rom filter (bilinear resampling blurred a pan's history frame after frame: 2.17%
  vs. 1.72% on the street at 1280x720), clips it toward the current 3x3 neighborhood's box (mean
  +- `CLIP_GAMMA` `1.5` standard deviations in YCoCg), and blends. The history's alpha counts its
  frames, so a pixel converges as a running mean until the current frame's weight reaches
  `temporal_blend` (`0.1`). Off-grid offsets, debug views, and the first frame after a toggle or
  resize (`VIEW_HISTORY_VALID` unset) take the current frame alone.
- **Present (`temporal_present.wesl`):** each view target pixel shows the resolved traced pixel it
  falls in, as lighting did without the resolve.
- **State:** `prepare_temporal_targets` keeps two `Rgba16Float` history images per view (alternating
  each frame) in a `Local`, dropped when the setting is off, the grid resizes, or the view goes;
  lighting's two targets come from the texture cache. The lighting pipeline key and present
  pipeline follow the setting itself, because `Queue` runs before `Prepare`; the resolve and present
  passes run only once lighting's temporal pipeline has compiled.
- **Benchmark:** `gpu_temporal_aa_quality` (ignored, `just test-gpu gpu_temporal_aa_quality
  --ignored --release`, 25 s) runs 32 frames of the production passes in the four benchmark views
  at both grids, still and panning 0.25 degrees of yaw per frame (about 2 and 4 px per frame), and
  compares the last frame with the 4x4 reference at the final pose; PNGs go under the temp
  directory's `vanta-temporal-aa`. It also times the resolve kernel.

#### Measurements

Display-space color error against the 4x4 reference, per pixel (3x3 filtered in parentheses), in
percent; the one-ray column is the exact frame of `gpu_footprint_lod_quality`. Flicker is the mean
change between the last two still frames.

| View, grid         | One ray     | Still, 32 frames | Still after 1 / 4 / 8 / 16 | Flicker | Pan: one ray | Pan, 32 frames |
| ------------------ | ----------- | ---------------- | -------------------------- | ------- | ------------ | -------------- |
| street, 640x360    | 3.70 (1.61) | 1.18 (0.63)      | 3.78 / 1.82 / 1.21 / 1.16  | 0.53    | 3.60 (1.53)  | 2.39 (0.82)    |
| street, 1280x720   | 2.32 (1.18) | 0.77 (0.47)      | 2.39 / 1.16 / 0.78 / 0.75  | 0.35    | 2.25 (1.11)  | 1.72 (0.65)    |
| grass, 640x360     | 2.13 (1.12) | 0.75 (0.48)      | 2.18 / 1.08 / 0.74 / 0.73  | 0.32    | 2.19 (1.13)  | 1.63 (0.69)    |
| grass, 1280x720    | 1.27 (0.73) | 0.49 (0.33)      | 1.31 / 0.65 / 0.45 / 0.46  | 0.20    | 1.34 (0.76)  | 1.08 (0.49)    |
| overview, 640x360  | 4.87 (1.69) | 1.48 (0.65)      | 4.92 / 2.23 / 1.54 / 1.42  | 0.60    | 4.86 (1.68)  | 2.13 (0.69)    |
| overview, 1280x720 | 3.70 (1.34) | 1.49 (0.69)      | 3.77 / 1.76 / 1.29 / 1.38  | 0.51    | 3.67 (1.31)  | 1.97 (0.59)    |
| grove, 640x360     | 3.08 (1.33) | 1.18 (0.63)      | 3.16 / 1.57 / 1.08 / 1.11  | 0.45    | 3.09 (1.32)  | 2.00 (0.70)    |
| grove, 1280x720    | 2.00 (0.97) | 1.11 (0.66)      | 2.07 / 1.05 / 0.83 / 1.00  | 0.33    | 1.99 (0.96)  | 1.48 (0.56)    |

The resolve kernel takes 0.018 ms at 640x360 and 0.055 ms at 1280x720. Tuning, all from this
benchmark: `CLIP_GAMMA` 1.0 / 1.25 / 1.5 / 2.0 gave still errors of 0.97 / 0.83 / 0.77 / 0.72 on the
street at 1280x720 and identical pan errors; 1.5 keeps most of the gain with a tighter box against
ghosting. `temporal_blend` `0.0625` was worse than `0.1` in every row (overview 1280x720 still 1.74,
pan 2.09).

In game (BRP, RTX 3080, default half scale and full scale): spinning rotors close up, walking, and
turning show no visible trails, and toggling the setting, debug views, and render scale changes
(history reset) render correctly.

#### Decisions

- **Temporal, on by default:** it beats the one-ray frame in every view, grid, and motion measured,
  for a negligible cost.
- **Resolve at the traced resolution:** at the default half scale each traced pixel still covers 2x2
  target pixels; temporal upsampling is the second slice.

#### Open issues

- **History validity:** only neighborhood clipping rejects history; the spec asks for pose-frame
  identity (plus generation) or depth checks, which matter where clipping can't tell (similar
  colors across a disocclusion, thin features in motion). Animated objects are untested in the
  benchmark, whose scenes are static snapshots.
- **Still views settle above their 8-frame error** in the grove and overview at 1280x720 (0.83 at 8
  frames, 1.11 at 32), where clipping against one-ray neighborhoods trims thin foliage history;
  coverage-aware or wider clipping where the history agrees with the current frame is a lever.
- **Footprint LOD with the resolve** is unmeasured (jitter and cones interact: a resolve averages
  subpixel coverage, which may make the opaque filter's dilation matter less).
- One BRP launch exited silently (no log) shortly after the resolve was enabled while the scene
  loaded; it did not recur across the later sessions' toggles, launches with it on by default, or
  movement, and the cause is unknown.

## Next steps, one per session

1. **Anti-aliasing, second slice:** history validity and temporal upsampling (see
   [Anti-aliasing](#anti-aliasing)). Add an animated case to `gpu_temporal_aa_quality` (a
   showcase rotor spinning between frames: build each frame's snapshot at its pose time with the
   previous frame's transforms, as the renderer does) and a disocclusion case, then reject
   history by pose-frame identity or depth (the spec's history rule; lighting can write the hit's
   frame ID or depth alongside the offset) and measure. Then resolve to the view target's
   resolution instead of the traced grid (jitter in target pixels, accumulate each traced sample
   into the target pixels it lands near), so the default half scale stops showing 2x2 blocks;
   compare against references at target resolution, and keep it behind a setting until it beats
   point upsampling. BRP screenshots as before.
2. **Atmosphere.** Today the sky is a horizon/zenith gradient with `pow()` sun and moon lobes
   (`lib/view.wesl` `sky_color`, colors hand-tuned in `render/environment.rs`), fog is a flat mix,
   and night is nearly black (a weak moon, `NIGHT_AMBIENT`, no exposure control). Each step is
   verified with BRP screenshots at noon, sunset, twilight, and midnight:
   1. Physically based sky scattering: Rayleigh, Mie, and ozone with Hillaire-style transmittance
      and sky-view LUTs, so sunrise, sunset, and twilight colors and the zenith and horizon follow
      from the sun instead of hand-tuned values.
   2. The sun through the atmosphere: a disk with limb darkening and a Mie halo replacing the
      `pow()` spot, surface sun light colored by the transmittance so lighting matches the sky,
      and aerial perspective replacing the flat fog mix.
   3. Night and exposure: stars and a moon that actually lights the scene, and exposure (a
      time-of-day curve as the first slice, then auto exposure / eye adaptation) so night is
      darker than day but readable, which sky-light occlusion's dark caves will also need.
3. **Sky-light occlusion:** an occlusion term for sky light (caves are as bright as the open
   meadow).
4. **Phase 2 showcase and gate checks:** the dense jungle-to-crystal-cavern scene against the
   spec's target (1440p / 60 FPS within 4 GB of VRAM on the RTX 3080, the M1 Pro as lower tier):
   slow pans, close approaches, overlapping leaves, thin crystal facets, memory occupancy, and
   comparison with supersampled images.

## What comes next

Performance is the open problem for Phase 2: the showcase traces in about 110 ms at 1280x720 on
an M1 Pro (software broad phase) and 4.4 ms on an RTX 3080 (hardware broad phase), and grass and
foliage take about 170 DDA steps per pixel, which no broad phase removes. The levers are the
spec's: footprint LOD (closed out, opt-in: it saves time only where it costs quality), then
temporal filtering (the temporal anti-aliasing resolve is in; temporal upsampling would let the
traced grid stay below target resolution without its blocks). Register pressure (128 per thread on NVIDIA for the
software path, 80 with ray queries) is worth watching with `just shader-stats` as traversal grows.

The roadmap, in the spec's order:

- Phase 2: footprint LOD filtering with fitted SGGX summaries and partial transmission (done,
  opt-in), anti-aliasing (temporal resolve done and on; history validity and upsampling next), a physically based atmosphere (scattering LUTs, sun disk, aerial perspective,
  night sky and moonlight, exposure), and an occlusion term for sky light. The reference GPU is
  the RTX 3080 (Vulkan, hardware broad phase), with the M1 Pro (Metal, software broad phase) as
  the lower tier.
- Phase 3: wind-driven deformation (Tier 2), tighter motion envelopes, thousands of articulated
  groups (rerunning the organization comparison with the hardware broad phase, and TLAS rebuild
  vs. refit), and animation LOD.
- Phase 4: GI, temporal filtering, homogeneous media, transparent and refractive crystals;
  secondary rays can share the TLAS on capable adapters.
- Phases 5 to 7: simulation, copy-on-write editing, streaming, and a camera-relative world origin
  for very large worlds.
