# Architecture

Vantablock follows the engine design spec (v3.1): canonical asset geometry, its articulated assembly,
and animation state are the source of truth; bounds, hierarchies, LODs, caches, and visibility
records are derived and replaceable. This document maps that design onto the code, records what
is built, and records what Phase 1 measured and decided.

## Crates

Crates are grouped by layer, and dependencies only flow downward:
`platform`, `engine/voxel` ← `engine/baker` ← `game/common` ← `game/server`/`game/client` ← `app`.

| Crate                                      | Role                                                                                                  |
| ------------------------------------------ | ----------------------------------------------------------------------------------------------------- |
| `vantablock-platform` (`crates/platform`)  | Host services with no game knowledge: logging, tracing, persistent paths                              |
| `vantablock-voxel` (`crates/engine/voxel`) | Bevy-free geometry core: data model, memory contracts, local hierarchies, CPU reference, scene search |
| `vantablock-baker` (`crates/engine/baker`) | Bevy-free procedural generators, RON recipes, `vanta_baker`, CPU traversal benchmarks                 |
| `vantablock-common` (`crates/game/common`) | The game both peers run: protocol, asset library, world objects, world clock, player simulation       |
| `vantablock-server` (`crates/game/server`) | Authoritative scene population, world clock, connections                                              |
| `vantablock-client` (`crates/game/client`) | Input, prediction, UI, and the voxel renderer                                                         |
| `vantablock-app` (`crates/app`)            | Binaries, singleplayer orchestrator, client/server integration tests                                  |

The `engine` crates serve every peer and tool: the server bakes canonical assets through
`common`, and the client additionally bakes their GPU hierarchies with `voxel`.

## Data flow

```text
assets/shared/vanta/*.ron  (recipes)
  -> baker::bake_recipe -> CanonicalAssembly            (cached as .vanta, keyed by recipe + version)
  -> VantaAssetLibrary (common, both peers)             (+ AssetTemplate: envelopes, template mini-BVH)

server: SceneLayout (assets/server/scenes/*.ron) -> replicated WorldObject + ObjectPlacement + ObjectAnimation
        WorldClock (FixedUpdate) -> ClockSync messages

client: GpuGeometryLibrary (bake every module as a 64-tree and an octree into shared arenas,
                            pack their pages into storage slabs sized to the device)
        every frame: objects + clock -> PoseDriver -> SceneSnapshot (frames, instances, BVHs) -> GpuSceneData
        render: PrimaryVisibility (compute) -> DirectLighting (fullscreen) in Bevy's Core3d graph,
                each in a single-slab or multi-slab shader variant
```

## Spec concepts in code

| Spec concept                                    | Code                                                                     |
| ----------------------------------------------- | ------------------------------------------------------------------------ |
| Logical asset / canonical assembly              | `voxel::assembly::CanonicalAssembly`, `voxel::module::VoxelModule`       |
| Pose groups, motion domains, envelopes          | `PoseGroupTemplate`, `Motion`, `CanonicalAssembly::motion_envelopes`     |
| Deterministic animation from a shared driver    | `voxel::pose::PoseDriver`, replicated `ObjectAnimation` + world clock    |
| Template-shared mini-BVH (Option B)             | `voxel::scene::AssetTemplate::mini_bvh`                                  |
| Flattened pose-group leaves (Option A)          | `SceneAcceleration::FlattenedGroups`                                     |
| Per-instance refit mini-BVHs                    | `SceneAcceleration::InstanceRefit`, `SceneSnapshot::instance_bvhs`       |
| Resolved pose frame, instance-owned slot table  | `ResolvedPoseFrame`, `SceneSnapshot::slot_frames`                        |
| Out-of-envelope fallback                        | `InstanceRecord::uses_template`                                          |
| Previous frames and motion reconstruction       | `ResolvedPoseFrame::previous_position`, `previous_world_position` (WGSL) |
| 64-tree, `Node64`, leaf tag in `lod_meta`       | `voxel::hierarchy::tree64`, `voxel::node`, `lib/tree64.wesl`             |
| 8-way octree with `8^3` bricks (comparison)     | `voxel::hierarchy::octree`, `lib/octree.wesl`                            |
| Shared DDA rules for every hierarchy            | `voxel::hierarchy`, `lib/dda.wesl`                                       |
| Two-`u32` masks and prefix counts               | `voxel::mask`, `lib/mask.wesl`                                           |
| `8 + 24` logical handles, typed arenas          | `voxel::handle`, `voxel::arena`                                          |
| No block crosses a page; no page crosses a slab | `WordArena::alloc`, `residency::plan_slabs`                              |
| Adapter-aware slab residency                    | `voxel::residency`, `client::render::device`, `GpuGeometryLibrary`       |
| Single-slab compiled fast path                  | the `multi_slab` WESL feature in `lib/geometry.wesl`                     |
| Single-page root fast path                      | `VoxelRoot::single_page`, `node_address` in `lib/dda.wesl`               |
| Traversal-state alternatives                    | the `stackless` WESL feature in `lib/tree64.wesl`                        |
| Optional hardware broad phase (Vulkan TLAS)     | the `ray_query` WESL feature in `lib/scene.wesl`, `frame_boxes`          |
| Register, spill, and occupancy profiling        | `crates/game/client/tests/shader_statistics.rs` (`just shader-stats`)    |
| 16-byte LOD payload                             | `voxel::lod`                                                             |
| Exact integer terminal-cell identity            | `VoxelHit::cell`, the visibility buffer's cell target                    |
| Visibility buffer (`R32G32Uint` + `R32Float`)   | `client::render::gpu::view`, `primary_visibility.wesl`                   |
| `.vanta` files                                  | `voxel::format`                                                          |

## Correctness contracts and how they are enforced

- **Reference semantics.** `voxel::reference::check_hit` defines an acceptable hit: an occupied voxel
  the ray truly enters, at its entry distance, with nothing solid meaningfully closer. Ties and
  sub-ulp grazing contacts may resolve either way.
- **CPU traversal** of both hierarchies is tested against brute force on boundary torture rays
  (`crates/engine/voxel/tests/hierarchy_reference.rs`): node and brick boundaries, corners, edges,
  boundary planes, near-parallel rays, inside starts, shared subtrees, multi-page layouts. The two
  hierarchies must also agree with each other.
- **Scene search** is tested against exhaustive per-frame search for every organization and both
  hierarchies over hundreds of animated, rotated instances, and hits must reconstruct a world
  position inside their reported cell and a previous position that follows their frame
  (`crates/engine/voxel/tests/scene_equivalence.rs`).
- **GPU** shaders (every variant) are validated with naga and run on the real adapter against the
  CPU reference (`crates/game/client/tests/gpu_conformance.rs`, `just test-gpu`): masks
  bit-for-bit; both hierarchies by the reference rules on the single-page, multi-page, and
  four-slab paths; the stackless experiment bit-identical to the stack; whole-scene traversal on
  the real assets for every organization, hierarchy, and a library split over several slabs; the
  hardware broad phase where the adapter has ray queries; and motion reconstruction across a pose
  change. `WGPU_BACKEND` and `WGPU_ADAPTER_NAME` pick the adapter.

Entry cells are chosen by comparing `t` against plane distances, never by flooring a reconstructed
position; flooring misplaced near-parallel rays by an ulp and missed geometry.

## Phase 1 results

Phase 1 is complete on Metal and Vulkan: every deliverable of the spec's renderer feasibility
phase, including the optional hardware broad phase, is built and held to the reference, and each
comparison it asks for has been measured on both backends.

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

| Scene, 64-tree                 | Template mini-BVH | Instance refit | Flattened | Flattened, hardware |
| ------------------------------ | ----------------- | -------------- | --------- | ------------------- |
| Showcase, Metal                | 149 ms            | 106 ms         | 111 ms    | n/a                 |
| Grove, Metal                   | 231 ms            | 120 ms         | 109 ms    | n/a                 |
| Showcase, Vulkan (86 inst.)    | 7.95 ms           | 6.07 ms        | 7.62 ms   | 4.42 ms             |
| Grove, Vulkan (300 animated)   | 12.60 ms          | 7.08 ms        | 7.17 ms   | 5.30 ms             |

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

**Decision:** keep it as an optional accelerator behind `GpuCapabilities::ray_query`, never a
requirement (Metal has no ray queries in wgpu). It lives in the conformance and benchmark harness
for now (`Variant::RayQuery`, the `ray_query` WESL feature); wiring it into the renderer as the
default broad phase on capable adapters is the first Phase 2 performance lever.

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
- Metal and Vulkan agree on every conformance case; no backend-specific shader path was needed.

## What comes next

Performance is the open problem for Phase 2: the showcase traces in about 110 ms at 1280x720 on
an M1 Pro and 7.6 ms on an RTX 3080, and grass and foliage take about 170 DDA steps per pixel. The
first lever is the measured one: the hardware broad phase as the renderer's default on capable
adapters. Then the spec's: footprint LOD (summaries are baked but unused) and SGGX directional
summaries, then temporal filtering, which can now build on reconstructed motion. Register pressure
(128 per thread on NVIDIA) is worth watching with `just shader-stats` as traversal grows.

The roadmap, in the spec's order:

- Phase 2: wire the hardware broad phase into the renderer (per-frame TLAS in the render world,
  `ray_query` variants of primary visibility and shadow rays, software BVH4 as the fallback), then
  footprint LOD filtering, foliage filtering candidates, anti-aliasing, atmosphere, and an
  occlusion term for sky light (caves are as bright as the open meadow). The reference GPU is the
  RTX 3080 (Vulkan), with the M1 Pro (Metal) as the lower tier.
- Phase 3: wind-driven deformation (Tier 2), tighter motion envelopes, thousands of articulated
  groups (rerunning the organization comparison with the hardware broad phase, and TLAS rebuild
  vs. refit), and animation LOD.
- Phase 4: GI, temporal filtering, homogeneous media, transparent and refractive crystals;
  secondary rays can share the TLAS on capable adapters.
- Phases 5 to 7: simulation, copy-on-write editing, streaming, and a camera-relative world origin
  for very large worlds.
