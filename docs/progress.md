# Progress

Where the engine stands against the [design spec](./design.md): what each phase delivered, what
was measured and decided, what comes next, and known issues. [`architecture.md`](./architecture.md)
maps the code itself; this file is the one that changes as work lands.

## Status

Phase 1 (renderer feasibility and memory contracts) is complete: its gate passed on Metal (M1 Pro)
and Vulkan (RTX 3080), and the 64-tree, flattened pose groups, and, on adapters with ray queries,
the hardware broad phase are the runtime defaults (see [Phase 1 results](#phase-1-results)).

Phase 2 is under way. Footprint LOD filtering is done as an opt-in path
(`RenderSettings::footprint_lod`, off by default, since filtered frames still miss the quality
target in most views; see [Phase 2 results](#phase-2-results)). Directional summaries: the CPU
comparison showed the spec's `L1` axis proxy overestimates angular coverage everywhere; every LOD
record now holds an SGGX tensor fitted at bake time to measured coverage, which filtered hits
evaluate. It cut coverage bias in frames only slightly. Partial transmission (filtered hits blend
with a second record traced behind them) is built as a further opt-in
(`RenderSettings::footprint_transmission`): it improves every filtered frame and meets the
"filtered pixels no worse than exact" target with a two-pixel cone at half scale, but its second
trace costs more than exact traversal, so footprint LOD stays off by default. Anti-aliasing is
next.

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

**Done (opt-in).** Every primary ray carries a pixel cone, and where a 64-tree node's cell fits
inside the cone's footprint the ray hits or passes the node's baked summary instead of descending.
It is held to a CPU reference on every variant and ships behind `RenderSettings::footprint_lod`,
off by default.

- **CPU reference:** `voxel::lod::FootprintFilter` is a cone (`radius + spread * t` in world units;
  local rays are never renormalized, so a frame's direction length scales it into voxels) and a
  threshold. `tree64::trace_filtered` shares `trace`'s DDA loop: an occupied node whose cell edge
  fits the footprint's diameter is tested with the record's fitted SGGX opacity
  (`min(sqrt(w^T S w), 1)`, `LodSummary::opacity`; the spec's `L1` axis sum overestimates
  coverage, see "Directional summaries") and stops the ray when the threshold is below it. Filtered hits report the aligned corner as `cell`, the level as
  `VoxelHit::lod` (`0` is exact), and the summary's material. `VoxelRoot::trace_filtered` is the
  frame trace of `SceneSnapshot::intersect`; octrees have no summaries and are never filtered.
- **Shader variant:** the `footprint_lod` WESL feature. `lib/lod.wesl` mirrors `voxel::lod`, and
  `trace_tree64_filtered` and `trace_scene_filtered` (`lib/tree64.wesl`, `lib/scene.wesl`) thread
  the filter through both broad phases; the exact traces are the filtered ones with
  `exact_footprint()`. Only the feature reads the LOD records (geometry binding 11, which needs 15
  storage buffers per stage), so the exact kernels are unchanged: 128 registers software, 80 with
  ray queries; the footprint LOD primary visibility forms use 128 and 96.
- **Occupied boxes:** a filtered hit fills its node's occupied box, clipped to the root's occupied
  bounds, and a ray that misses it passes through. Clipping is required for correctness: every
  broad phase culls frames by those bounds (`PoseFrame::local_bounds_min`/`max`, 224 bytes), so an
  unclipped node box sticking out of them was hit by exhaustive search but not through the BVHs.
- **LOD record (16 bytes):** words 0 and 1 hold the fitted SGGX tensor's axis coverages
  `c_a = sqrt(S_aa)` (f16) and the material, word 2 its correlations `r_ab = S_ab / (c_a c_b)`
  (three 10-bit offset snorms), word 3 the occupied box (4 bits per bound in sixteenths of the node
  edge, rounded outward) and the solid volume fraction (unorm8). The material is the one with the
  most exposed faces; faces count as exposed only when the neighbor voxel in the module is empty.
  `.vanta` files hold canonical modules, so the record format needs no `BAKER_VERSION` bump.
- **Primary visibility:** `pixel_footprint` uses no radius and a spread of half the traced pixel's
  angle times `footprint_scale` (`1`), so reduced render scales filter more; `footprint_threshold`
  is `0.5`. Both reach the shader through `ViewUniform`. `vis.x` carries the hit's level and `vis.y`
  the LOD record index. `PrimaryVisibilityPipeline` holds an exact and a footprint LOD variant set,
  chosen per frame: a runtime switch between pipelines rather than a uniform in the exact kernels.
  `F8`/`L` toggles it (pause menu, debug overlay), and the `LodLevels` debug view shows exact voxels
  grey and filtered levels from blue to red.
- **Stochastic thresholds:** `RenderSettings::footprint_stochastic` (off) sets
  `VIEW_STOCHASTIC_FOOTPRINT`, which takes each pixel's threshold from a stable PCG hash, so a
  summary of opacity `a` stops a fraction `a` of the rays.
- **Filtered-hit shading:** `filtered_face_light` (`direct_lighting.wesl`) averages the sun cosine,
  sky height, and ground bounce of the node's three camera-facing faces, each weighted by the
  share of the view it covers (`|d_a|` times its node-relative coverage, `lod_node_coverage`), as
  exact voxels' faces average over a footprint. The one shadow ray leaves the occupied box toward
  the light, so the node's own voxels can't shadow it.
- **Benchmarks** (ignored tests in `gpu_conformance.rs`, run with `--release --nocapture`):
  `gpu_primary_visibility_cost` dispatches the production primary visibility kernels (both broad
  phases) in four views (street, grass level, overview, the 300-instance grove from 48 m) at render
  scales 0.5 and 1.0, exact and with one- and four-pixel cones, and reports kernel time, DDA steps,
  and the filtered share. `gpu_footprint_lod_quality` renders the same views through both
  production passes and compares one ray per pixel (exact, and 1, 2, and 4 px cones, fixed and
  stochastic thresholds) with a 4x4 supersampled exact reference: per-pixel and 3x3-filtered color
  error, coverage bias, and the error on just the filtered pixels next to the exact frame's on the
  same pixels. Frames are written as PNGs under the temp directory's `vanta-footprint-quality`.
  Both also run the footprint transmission variants (see "Partial transmission"), and the cost
  benchmark reports the share of pixels that traced a second record.
- **Tests:** the CPU reference (`tree64` units,
  `footprint_filtering_reports_aligned_occupied_regions`,
  `filtered_scene_traces_match_exhaustive_search_in_world_units`) and the GPU conformance tests
  (`gpu_footprint_filtering_matches_cpu`, `gpu_filtered_scene_traversal_matches_cpu`: cell, level,
  material, `t`, and the decoded correlations, every organization, hierarchy, residency, and ray query
  variant) agree with zero disagreements. Cones that exactly fit a cell are ties `length(d)` can
  round either way, so the tests avoid exact fits.

Measured at the final state: four-pixel cone vs. exact, hardware broad phase / software BVH4 kernel
time, and display-space color error against the reference as `per pixel / 3x3 / coverage bias`
(points), with the filtered pixels' error vs. the exact frame's on the same pixels (fitted SGGX
records):

| View, grid         | Exact time (ms) | 4 px time                 | Exact error         | 4 px error          | Filtered px: 4 px vs. exact |
| ------------------ | --------------- | ------------------------- | ------------------- | ------------------- | --------------------------- |
| street, 640x360    | 1.87 / 2.68     | -29 to -39% / -28 to -34% | 3.70 / 1.61 / +0.01 | 4.20 / 2.22 / +0.93 | 5.76% vs. 4.50%             |
| street, 1280x720   | 3.41 / 5.33     | within noise              | 2.32 / 1.18 / +0.01 | 2.50 / 1.37 / +0.38 | 6.28% vs. 4.02%             |
| grass, 640x360     | 0.98 / 1.07     | -5 to -10% / -8 to -14%   | 2.13 / 1.12 / 0.00  | 2.46 / 1.52 / +0.60 | 5.25% vs. 3.75%             |
| overview, 640x360  | 1.29 / 1.51     | -13 to -22% / -24 to -25% | 4.87 / 1.69 / -0.02 | 4.83 / 2.12 / +0.42 | 6.69% vs. 6.77%             |
| overview, 1280x720 | 2.30 / 3.51     | -16 to -21% / -19 to -22% | 3.70 / 1.34 / 0.00  | 4.26 / 2.03 / +0.40 | 6.21% vs. 5.27%             |
| grove, 640x360     | 1.39 / 1.83     | -8 to -9% / -16 to -18%   | 3.08 / 1.33 / +0.02 | 3.85 / 2.18 / +1.09 | 5.39% vs. 4.16%             |
| grove, 1280x720    | 4.35 / 4.75     | within noise              | 2.00 / 0.97 / 0.00  | 2.22 / 1.18 / +0.29 | 4.33% vs. 3.07%             |

(Reference coverage: street 90.6%, grass level 84.2%, overview 70.5%, grove 74.6%. Time ranges
span the `L1` axis, `L2` axis, and fitted SGGX runs, which agree within noise; dispatch noise is
about 8%, more at 1280x720 on the hardware path. At a one-pixel
cone at most 8% of hits are filtered at half scale, the frame is within 0.04 points of exact, and
the cost is 0 to 12%, mostly noise. At two pixels the overview frame is 4.50% per pixel.)

What it says: a four-pixel cone saves up to 39% of primary visibility time wherever a view reaches
distance, less at grass level, where the expensive pixels are close; time falls faster than mean
DDA steps, so the savings come from the long-tail pixels. On the grove the hardware path saves
little, because rays that miss a small occupied box keep traversing. In the overview, filtered
pixels and the whole frame per pixel beat the exact ray; elsewhere filtered pixels carry 1.3 to 2.3
points more error than exact, and every filtered frame's 3x3 error is 0.2 to 0.9 points above the
exact frame's.

Dead ends, so nobody retries them: shading with the summary's mean normal made filtered pixels
worse (it describes the node's net orientation, not the faces the camera sees; the record
no longer stores it). Self-shadowing was not the cause of dark filtered
crystals (moving the shadow ray out of the box gained 0.05 to 0.1 points), nor was the single
material (0.01 to 0.04). The occupied box with node-relative coverage cut coverage bias further
(overview 4 px +0.26) but left higher color error than box-relative coverage.

**Decisions:**

- **`footprint_lod` is off by default,** with a one-pixel cone: at one pixel it changes nothing,
  and larger cones miss the target (filtered pixels no worse than exact) in street, grass level,
  and grove views. It stays a runtime toggle for distance and reduced render scales.
- **Axis coverage is relative to the occupied box,** a departure from the spec's node-relative
  coverage: the opacity proxy is the chance of a hit given that the ray enters the box, and
  node-relative coverage (the box value times the box face's share) weights shading.
- **Thresholds stay deterministic (`0.5`);** stochastic thresholds trade bias for noise that only
  an averaging or temporal resolve removes, and now move results by at most 0.2 points, since
  box-relative opacities sit near 0 or 1.

**Open issues:**

- **Coverage bias:** filtered frames still dilate silhouettes (+0.3 to +1.1 points at 4 px), a
  low-frequency error the 3x3 filter doesn't remove. The fitted SGGX tensor removed only 0.01 to
  0.2 points of it, so the remaining bias isn't the opacity estimate but its use as a hit/pass
  decision.
- **Partial transmission:** built and measured, see "Partial transmission of filtered hits": it
  closes much of the quality gap but costs more than it saves.
- **Grove hardware-path cost:** small occupied boxes leave the hardware broad phase saving -5 to
  +1% on the grove, against -18 to -28% on the software BVH4.
- The footprint rule (cell edge vs. cone diameter) is untuned, and the 4x4 reference is itself
  limited: the exact one-ray frame is 2 to 5% from it, mostly grass and foliage aliasing.

### Directional summaries: axis proxy vs. SGGX

**Done: every LOD record holds an SGGX tensor fitted to measured coverage, and filtered hits
evaluate it.** `voxel::angular`
measures a node's angular coverage (the chance that a ray along `w` entering the occupied box hits
a voxel, the quantity a filtered hit stands for) with 48x48 stratified parallel rays per
direction, and scores four summaries baked from the same voxels on 96 directions disjoint from
the 48 the fits see: the spec's axis proxy (`clamp(sum(|w_a| c_a))`), the diagonal tensor
`diag(c_a^2)` (the same three coverages combined in `L2`: no extra bytes), a PSD SGGX tensor
least-squares fitted to measured `coverage^2` and stored as six `f16` (12 bytes), and a
15-coefficient quartic (spherical harmonic bands 0, 2, 4) as the richer reference. The study is
the ignored test `directional_lod_comparison` (`cargo test --release -p vantablock-baker --test
directional_lod -- --ignored --nocapture`, 8 s); it also scores an axis-fixed fit and the
baked tensor (below).

Mean `|error|` (bias) and the share of node-direction pairs whose hit/pass decision at the `0.5`
threshold differs from the measurement, for up to 160 nodes per asset and size:

| Nodes                    | Axis proxy           | Diagonal SGGX        | Fitted SGGX (f16)   | Quartic            |
| ------------------------ | -------------------- | -------------------- | ------------------- | ------------------ |
| diagonal/tilted leaves   | 0.442 (+0.44), 48.6% | 0.259 (+0.26), 44.8% | 0.057 (+0.02), 5.9% | 0.023 (0.00), 2.3% |
| oak 4^3                  | 0.156 (+0.16), 5.0%  | 0.058 (+0.05), 3.6%  | 0.015 (0.00), 1.2%  | 0.007 (0.00), 0.7% |
| oak 16^3                 | 0.289 (+0.29), 38.1% | 0.104 (+0.10), 21.7% | 0.025 (0.00), 5.0%  | 0.012 (0.00), 2.2% |
| oak 64^3                 | 0.266 (+0.27), 45.3% | 0.090 (+0.09), 11.4% | 0.022 (0.00), 1.8%  | 0.011 (0.00), 1.1% |
| crystal grotto 4^3       | 0.120 (+0.12), 0.6%  | 0.045 (+0.04), 0.6%  | 0.011 (0.00), 0.3%  | 0.005 (0.00), 0.1% |
| crystal grotto 16^3      | 0.274 (+0.27), 20.9% | 0.122 (+0.09), 18.9% | 0.031 (0.00), 5.1%  | 0.017 (0.00), 2.6% |
| crystal grotto 64^3      | 0.349 (+0.35), 43.4% | 0.161 (+0.13), 32.2% | 0.040 (+0.01), 6.9% | 0.023 (0.00), 4.2% |
| meadow tile 4^3          | 0.144 (+0.14), 14.1% | 0.045 (+0.04), 4.3%  | 0.013 (0.00), 1.8%  | 0.008 (0.00), 1.1% |
| meadow tile 16^3         | 0.212 (+0.21), 29.8% | 0.075 (+0.07), 3.3%  | 0.019 (0.00), 0.4%  | 0.011 (0.00), 0.4% |
| meadow tile 64^3         | 0.263 (+0.26), 0.0%  | 0.086 (+0.09), 0.0%  | 0.015 (0.00), 0.0%  | 0.010 (0.00), 0.0% |

(Sampling error is about 1% per direction, so the quartic sits near the noise floor.)

What it says: the axis proxy fails the spec's test. It overestimates coverage in every population,
by 0.12 to 0.35 on average, and at the deterministic `0.5` threshold it decides 21 to 45% of the
16^3 and 64^3 node-direction pairs wrongly, which is the measured coverage bias (silhouette
dilation) of filtered frames. `L1` is the culprit: the same three coverages combined in `L2` cut
the error 1.7 to 3.2x for free. The fitted SGGX tensor cuts it 8 to 18x, is unbiased, and its
mean error stays within 0.035 of the quartic's everywhere, so the quartic's 30 bytes aren't worth
it. A tensor built from
exposed faces instead of fitted is diagonal for voxels (their faces are axis aligned) and can't
tell a leaf along `x = y` from one along `x = -y`; fitting to measured coverage is what captures
diagonal leaves and crystal facets.

Fixing the fit's diagonal to the exact axis coverages (`Sggx::fit_with_axes`, fitting only the
correlations) keeps nearly all of the axis proxy's bias (oak 16^3: 0.100 vs. 0.104 for the
diagonal), because the bias is in the diagonal: true coverage off the axes is lower than the axes
imply. The bake therefore fits all six terms, and the record's axis coverages are the fit's.

**Baked tensor (shipped):** `BakeSetup::tensor` (`bake_setup()` for `BAKE_MEASUREMENT`) measures
16 directions and least-squares fits all six terms. Bricks use `BrickRays`: per direction, the
distinct voxel sets that 64x64 jittered lines through a brick cross (about 360 per direction),
weighted by line count, so a brick's coverage is a weighted count of the sets meeting its mask
over those meeting its occupied box (measured once per distinct occupancy). Larger nodes march
8x8 stratified rays per direction through a sparse brick-mask `NodeVoxels` that skips empty
bricks. `tree64::bake` measures every distinct brick mask and every internal node on all cores
before building (`NodeTensors`). The record stores `c_a = sqrt(S_aa)` as f16 and the
correlations as 10-bit snorms (`lod::stored_tensor`); the octahedral normal and its spread are
gone. `lod_opacity` (`lib/lod.wesl`) and `LodSummary::opacity` evaluate
`min(sqrt(max(w^T S w, 0)), 1)`; shading's node-relative face coverages use the fit's `c_a`.
Conformance checks the decoded correlations on every variant,
`baked_tensors_match_the_directional_study` holds the record to the study's "baked SGGX" column,
and `brick_line_sets_measure_what_rays_do` holds `BrickRays` to ray marching.

Baked accuracy (the study's "baked SGGX", the record as stored), mean `|error|` and `0.5` flips,
next to the 48-direction, 48x48-ray fit: diagonal and tilted leaves 0.071 / 9.5% (0.057 / 5.9%);
oak 4^3 0.016 / 1.5%, 16^3 0.036 / 7.7%, 64^3 0.034 / 2.5%; crystal grotto 4^3 0.012 / 0.3%,
16^3 0.040 / 7.4%, 64^3 0.051 / 9.5%; meadow tile 4^3 0.014 / 1.9%, 16^3 0.031 / 1.1%, 64^3
0.025 / 0.0%. Bias stays within +-0.005. 12x12 rays for internal nodes gain about 0.006 for 15%
more bake time; 12 directions with 6x6 rays lose about 0.01.

Costs (16 threads): the 64-tree bake takes 52 / 260 / 14 / 175 / 72 ms for the crystal cluster,
crystal grotto, kinetic rotor, meadow tile, and oak, against 37 / 213 / 10 / 141 / 48 ms with
the axis record (single-threaded DDA measurement took seconds). Dropping the normal improves
dedup: LOD records are 7,387 / 52,860 / 1,160 / 34,154 / 16,090 (was 8,345 / 60,795 / 1,392 /
31,826 / 18,430), 16 bytes each. Registers are unchanged (footprint LOD primary visibility 128
software, 96 with ray queries).

In frames (`gpu_footprint_lod_quality`, 4 px, the table in "Footprint LOD filtering") the tensor
lowers coverage bias 0.01 to 0.21 points against the `L2` axis proxy (most in the overview, +0.63
to +0.42) and 3x3 error up to 0.05 points; per-pixel frame error moves within +-0.1 points
(overview +0.1) and filtered pixels' error by -0.09 to +0.12. As with `L2`, box-relative
opacities sit near 0 or 1, so the better estimate flips few hit/pass decisions: the remaining
error is the binary decision, not the estimate. Primary visibility cost is unchanged within noise.

### Partial transmission of filtered hits

**Done (opt-in, off).** With `RenderSettings::footprint_transmission` (and `footprint_lod`), a
filtered hit is no longer all or nothing: it covers its pixel by its summary's opacity `a` (the
fitted SGGX value along the ray), and the ray continues past the node's occupied box to a second
visibility record, which lighting blends in with weight `1 - a`.

- **Primary visibility:** the `transmission` WESL feature (three more primary visibility builds:
  single slab, multi slab, ray query; `PrimaryVisibilityPipeline::footprint_transmission`). The
  front trace uses threshold `0`, so every summary the cone covers with any opacity becomes the
  front record (`0` and `0.25` measured identically). `continuation` computes `a` and the exit of
  the clipped occupied box in the frame's local ray; if `a < transmission_opaque` (`0.9`,
  `ViewUniform`), a second `trace_scene_filtered` starts just past the exit (`1e-5` relative, so
  the same node can't be hit again) with the pixel's own `0.5` threshold. Both traces share one
  call site in a two-iteration loop with only the packed front record live across it
  (`trace_pixel`).
- **Visibility buffer:** every target holds a second record: `vis` is `Rgba32Uint` (front, behind
  identity and handle), `depth` `Rgba32Float` (front t, behind t, coverage `a`, 1 for opaque and
  exact hits), `cell` `Rg32Uint`. Exact and footprint LOD kernels write `a = 1` and a missing
  record behind, and keep their register counts (128 / 80 exact, 128 / 96 footprint LOD).
- **Lighting:** `shade` shades both records with `shade_record` (each with its own shadow ray and
  fog) and mixes them by `a`; a missing record behind is the sky. Debug views show the front
  record.
- **Why a separate variant:** any second trace site in the footprint LOD kernel (two calls or the
  loop) raised it from 128 to 168 registers (96 to 128 with ray queries; the driver moves the
  private stacks from local memory into registers) and cost the plain filtered path about ten
  points of its savings even with the second trace never taken.

Quality (`gpu_footprint_lod_quality`, hardware broad phase), `per pixel / 3x3 / coverage bias` and
filtered pixels' error vs. the exact frame's on the same pixels, transmission vs. the opaque
decision (the `exact` frames are in the "Footprint LOD filtering" table):

| View, grid         | 2 px opaque         | 2 px transmission   | 4 px opaque         | 4 px transmission   | Filtered px, 2 px / 4 px transmission |
| ------------------ | ------------------- | ------------------- | ------------------- | ------------------- | ------------------------------------- |
| street, 640x360    | 3.76 / 1.70 / +0.38 | 3.67 / 1.64 / +0.27 | 4.20 / 2.22 / +0.93 | 3.89 / 2.00 / +0.66 | 5.48 vs. 5.86 / 5.01 vs. 4.53         |
| street, 1280x720   | 2.32 / 1.18 / +0.01 | 2.32 / 1.18 / +0.01 | 2.50 / 1.37 / +0.38 | 2.43 / 1.30 / +0.26 | 3.21 vs. 3.02 / 5.47 vs. 4.05         |
| grass, 640x360     | 2.17 / 1.19 / +0.28 | 2.10 / 1.15 / +0.21 | 2.46 / 1.52 / +0.60 | 2.31 / 1.40 / +0.45 | 4.80 vs. 5.22 / 4.58 vs. 3.78         |
| overview, 640x360  | 4.71 / 1.87 / +0.41 | 3.83 / 1.57 / +0.27 | 4.83 / 2.12 / +0.42 | 3.72 / 1.68 / +0.33 | 5.04 vs. 6.82 / 5.17 vs. 6.78         |
| overview, 1280x720 | 3.70 / 1.36 / +0.03 | 3.65 / 1.34 / +0.02 | 4.26 / 2.03 / +0.40 | 3.53 / 1.64 / +0.27 | 3.56 vs. 4.43 / 4.98 vs. 5.27         |
| grove, 640x360     | 3.15 / 1.42 / +0.31 | 3.05 / 1.37 / +0.24 | 3.85 / 2.18 / +1.09 | 3.44 / 1.89 / +0.83 | 4.19 vs. 4.38 / 4.75 vs. 4.17         |
| grove, 1280x720    | 2.01 / 0.98 / +0.03 | 2.01 / 0.97 / +0.02 | 2.22 / 1.18 / +0.29 | 2.14 / 1.12 / +0.21 | 3.38 vs. 3.19 / 3.90 vs. 3.08         |

Cost (`gpu_primary_visibility_cost`, 640x360, hardware / software, against exact): 2 px
transmission +1% / +16% (street), +34% / +33% (grass), +83% / +44% (overview), +44% / +35%
(grove); 4 px transmission +25% / +19%, +45% / +52%, +78% / +46%, +59% / +61%, where the opaque
4 px cone saves 34% / 26% on the street. 5 to 9% of pixels trace a second record at 2 px (47% in
the overview), 15 to 59% at 4 px. Opaque cutoffs of `0.995` and `0.75` instead of `0.9` changed
quality by +0.0 / -0.1 points and time by +10% / -20%.

What it says: transmission improves every filtered frame, per pixel, after the 3x3 filter, and in
coverage bias (by a quarter to a third), and the overview's 4 px frame beats the exact one by 1.15
points. At 2 px and half scale filtered pixels beat exact ones in every view, the target; at 4 px
they still trail by 0.5 to 0.8 points in the street, grass, and grove views, and at 2 px and full
scale by 0.2 on the few filtered street and grove pixels. But the second trace restarts scene
search from the camera and diverges within warps, so every transmission configuration costs more
than exact traversal: footprint LOD with transmission is a quality fix, not a speed-up.

**Decisions:** `footprint_transmission` ships off (`transmission_opaque` `0.9`), and
`footprint_lod` stays off by default: the opaque decision is the fast path, transmission the
accurate one, and neither is both. Stochastic thresholds stay as they were (biased low and noisy;
see "Footprint LOD filtering").

**Open issues:** the remaining 4 px gap and dilation (+0.2 to +0.8 points) are what a single
summary per node can't express (a node partly covering the cone; the cone rule itself is
untuned). The cost lever, if transmission is revisited, is to trace the records behind in a
second, compacted dispatch (only transmitting pixels, resuming from the front frame instead of
the scene root) rather than in the primary kernel.

## Next steps, one per session

1. **Anti-aliasing:** the exact one-ray frame is 2 to 5% per pixel from the 4x4 supersampled
   reference (`gpu_footprint_lod_quality`'s `exact` rows, mostly grass and foliage aliasing), so
   that benchmark is the yardstick. First slice: decide between jittered primary rays with a
   temporal resolve (reconstructed motion exists: the `Motion` debug view) and a spatial filter
   over the visibility buffer, measure the chosen one against the reference at both grids, and
   keep it behind a `RenderSettings` flag until it beats the one-ray frame. Check it in game with
   BRP screenshots (moving camera and animated rotors, so ghosting shows).
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
spec's: footprint LOD (built, opt-in, with fitted SGGX summaries), then temporal filtering,
which can build on reconstructed motion. Register pressure (128 per thread on NVIDIA for the
software path, 80 with ray queries) is worth watching with `just shader-stats` as traversal grows.

The roadmap, in the spec's order:

- Phase 2: footprint LOD filtering (done, opt-in), foliage filtering candidates (fitted SGGX
  baked and evaluated; partial transmission built, opt-in),
  anti-aliasing, a physically based atmosphere (scattering LUTs, sun disk, aerial perspective,
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
