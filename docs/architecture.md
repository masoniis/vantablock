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
  the real assets for every organization, hierarchy, and a library split over several slabs; and
  motion reconstruction across a pose change.

Entry cells are chosen by comparing `t` against plane distances, never by flooring a reconstructed
position; flooring misplaced near-parallel rays by an ulp and missed geometry.

## Phase 1 results

Phase 1 is complete on Metal: every deliverable of the spec's renderer feasibility phase is built
and held to the reference, and each comparison it asks for has been measured. All numbers are from
an M1 Pro; GPU times are 1280x720 primary rays including readback (`gpu_scene_throughput`,
`gpu_multi_slab_cost`), CPU times from `just bench`.

**Gate.** Both hierarchies produce the reference's hits with no missed geometry, rotated pose
groups stay hittable, and visibility reconstructs exact terminal-cell identity, world position,
and motion, including across deduplicated geometry and slab boundaries. Template-shared mini-BVHs
are correct for every legal pose and fall back safely outside their envelopes.

### Local hierarchy: 64-tree vs. 8-way octree with `8^3` bricks

| Asset                      | 64-tree topology / attributes | Octree topology / attributes |
| -------------------------- | ----------------------------- | ---------------------------- |
| oak (8 modules)            | 331 KiB / 1558 KiB            | 324 KiB / 1769 KiB           |
| meadow tile (1024^3 grass) | 1403 KiB / 1144 KiB           | 1018 KiB / 5244 KiB          |
| crystal grotto (6 modules) | 1339 KiB / 2356 KiB           | 1195 KiB / 5961 KiB          |

The octree's topology is a little smaller, but its 512-voxel attribute blocks deduplicate far less
than the 64-tree's 64-voxel ones, so it needs up to 4.6x the attribute memory on repetitive
foliage. On the CPU reference the octree traces 1.25 to 1.75x faster; on the GPU it is 26 to 41%
slower in every scene and organization (e.g. 152 vs. 109 ms on the grove, flattened).

**Decision:** the 64-tree stays the default; the octree remains selectable at runtime (`H`/`F6`)
and in every test. The sparse brick grid was not built: the spec needs one comparison, and the
octree already isolates the effect of fan-out and brick size.

### Pose-group organization: template mini-BVH vs. refit vs. flattened

| Scene, 64-tree                 | Template mini-BVH | Instance refit | Flattened |
| ------------------------------ | ----------------- | -------------- | --------- |
| Showcase (86 instances)        | 149 ms            | 106 ms         | 111 ms    |
| Grove (300 animated instances) | 231 ms            | 120 ms         | 109 ms    |

On the grove, the template's shared mini-BVHs take 1.4 KiB but its rays visit 28.9 scene nodes, 69
mini-BVH nodes, and 7.8 candidate groups each, because instances are bounded by sphere-swept
motion envelopes. Refitting costs 138 KiB of per-instance nodes and visits 12.8 scene nodes, 15.6
mini-BVH nodes, and 6.5 groups; flattening costs a 152 KiB scene BVH and visits 21.2 nodes and 6.6
groups. Every snapshot builds in under 0.6 ms.

**Decision:** flattened groups are the runtime default (`B`/`F4` cycles all three). Template
mini-BVHs are correct but lose on loose envelopes; tighter envelopes (sweeping only a hinge's legal
arc rather than a sphere) and thousands of foliage groups are Phase 3 questions, where this
comparison should be rerun.

### Residency: single slab vs. multi-slab

Pages are packed whole into at most four slabs per arena. Reading through a slab switch at every
node fetch cost the DDA loop about 10% even when everything was in one slab, so every shader
compiles a single-slab variant (direct indexing) and a multi-slab one, chosen from the uploaded
library. Spread over two topology and four attribute slabs, the grove traces 11% (template) to 19%
(flattened) slower than with the fast path.

**Decision:** keep the `8 + 24` handle format and slab-address page tables; the multi-slab path is
correct and its cost is known.

### Traversal state: short private stack vs. stackless restart

The stackless variant keeps no parent state and re-walks the packed path from the root on each
pop. It is bit-identical to the stack, and 12 to 29% slower in every scene and organization.

**Decision:** keep the short private stack. wgpu exposes no register, spill, or occupancy counts
on Metal; profiling those needs Xcode's GPU tools or a Vulkan adapter, and the decision should be
revisited with them.

### Not covered on this hardware

- **Vulkan.** Every test and measurement above ran on Metal. The suite is backend-agnostic and
  should be rerun on a Vulkan adapter before the formats are frozen there.
- **Hardware ray queries.** wgpu offers no ray queries on Metal (`GpuCapabilities::ray_query` is
  false on the M1); the optional hardware broad phase waits for a Vulkan adapter.

## What comes next

Performance is the open problem for Phase 2: the showcase traces in about 110 ms at 1280x720, and
grass and foliage take about 170 DDA steps per pixel. The spec's levers are footprint LOD (summaries
are baked but unused) and SGGX directional summaries, then temporal filtering, which can now build
on reconstructed motion.

Later phases, in the spec's order:

- Phase 2: footprint LOD filtering, foliage filtering candidates, anti-aliasing, atmosphere, and an
  occlusion term for sky light (caves are as bright as the open meadow).
- Phase 3: wind-driven deformation (Tier 2), tighter motion envelopes, thousands of articulated
  groups, and animation LOD.
- Phase 4: GI, temporal filtering, homogeneous media, transparent and refractive crystals.
- Phases 5 to 7: simulation, copy-on-write editing, streaming, and a camera-relative world origin
  for very large worlds.
