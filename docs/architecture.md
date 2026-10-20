# Architecture

Vantablock follows the engine design spec (v3.1): canonical asset geometry, its articulated assembly,
and animation state are the source of truth; bounds, hierarchies, LODs, caches, and visibility
records are derived and replaceable. This document maps that design onto the code. It changes when
the structure does; what has been measured, decided, and is next lives in
[`progress.md`](./progress.md), and the design itself in [`design.md`](./design.md).

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
                            pack their pages into storage slabs sized to the device, and
                            upload the LOD summary records as one binding)
        every frame: objects + clock -> PoseDriver -> SceneSnapshot (frames, instances, BVHs) -> GpuSceneData
                     (+ one box per pose frame where the hardware broad phase applies)
        render: SceneTlas (ray query adapters) -> PrimaryVisibility (compute) -> DirectLighting
                (fullscreen) in Bevy's Core3d graph, each in a single-slab, multi-slab, or
                hardware broad phase shader variant; primary visibility also has footprint LOD
                and footprint transmission forms of all three (`RenderSettings::footprint_lod`,
                `footprint_transmission`)
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
| Optional hardware broad phase (Vulkan TLAS)     | `ray_query` in `lib/scene.wesl`, `frame_boxes`, `gpu::broad_phase`       |
| Register, spill, and occupancy profiling        | `crates/game/client/tests/shader_statistics.rs` (`just shader-stats`)    |
| 16-byte LOD payload (fitted SGGX, box, material) | `voxel::lod`, the bake-time fit in `voxel::angular::BakeSetup`           |
| Directional LOD candidates (axis, SGGX) study   | `voxel::angular`, `crates/engine/baker/tests/directional_lod.rs`         |
| Footprint LOD filtering (primary rays, opt-in)  | `tree64::trace_filtered`, `VoxelRoot::trace_filtered`, `trace_scene_filtered`, the `footprint_lod` WESL feature, `pixel_footprint` in `primary_visibility.wesl` |
| Partial transmission of filtered hits (opt-in)  | the `transmission` WESL feature (`trace_pixel`, `continuation` in `primary_visibility.wesl`), the blend in `direct_lighting.wesl` `shade` |
| Exact integer terminal-cell identity            | `VoxelHit::cell`, the visibility buffer's cell target                    |
| Visibility buffer (front + behind records: `Rgba32Uint`, `Rgba32Float`, `Rg32Uint`) | `client::render::gpu::view`, `primary_visibility.wesl`                   |
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
  four-slab paths; the stackless experiment bit-identical to the stack; 64-tree footprint LOD
  filtering on the real assets taking the CPU reference's exact path (hit, level, material, and
  step count) and decoding the SGGX correlations as the CPU does, in both residency layouts, and whole-scene filtered traversal finding the CPU's
  nearest filtered hits for every organization, including the hardware broad phase; whole-scene traversal on
  the real assets for every organization, hierarchy, and a library split over several slabs; the
  hardware broad phase where the adapter has ray queries; and motion reconstruction across a pose
  change. `WGPU_BACKEND` and `WGPU_ADAPTER_NAME` pick the adapter.

Entry cells are chosen by comparing `t` against plane distances, never by flooring a reconstructed
position; flooring misplaced near-parallel rays by an ulp and missed geometry.
