# Progress

Where the engine stands against the [design spec](./design.md): what each phase delivered, what
was measured and decided, what comes next, and known issues. [`architecture.md`](./architecture.md)
maps the code itself; this file is the one that changes as work lands.

## Status

Phase 1 (renderer feasibility and memory contracts) is complete on Metal and Vulkan; Phase 2 is
next.

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
