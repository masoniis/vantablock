# Architecture

Vantablock follows the engine design spec (v3.1): canonical asset geometry, its articulated assembly,
and animation state are the source of truth; bounds, hierarchies, LODs, caches, and visibility
records are derived and replaceable. This document maps that design onto the code and records what
Phase 1 implements.

## Crates

| Crate               | Role                                                                                         |
| ------------------- | -------------------------------------------------------------------------------------------- |
| `vantablock-voxel`  | Bevy-free geometry core: data model, memory contracts, CPU reference traversal, scene search |
| `vantablock-baker`  | Procedural generators, RON recipes, `vanta_baker`, CPU traversal benchmarks                  |
| `vantablock-shared` | Network protocol, asset library, replicated world objects, world clock, player simulation    |
| `vantablock-server` | Authoritative scene population, world clock, connections                                     |
| `vantablock-client` | Input, prediction, UI, and the voxel renderer                                                |
| `vantablock-runner` | Binaries, singleplayer orchestrator, client/server integration tests                         |
| `vantablock-utils`  | Logging and persistent paths                                                                 |

Dependencies point one way: `voxel` ← `baker` ← `shared` ← `server`/`client` ← `runner`.

## Data flow

```text
assets/shared/vanta/*.ron  (recipes)
  -> baker::bake_recipe -> CanonicalAssembly            (cached as .vanta, keyed by recipe + version)
  -> VantaAssetLibrary (shared, both peers)             (+ AssetTemplate: envelopes, template mini-BVH)

server: SceneLayout (assets/server/scenes/*.ron) -> replicated WorldObject + ObjectPlacement + ObjectAnimation
        WorldClock (FixedUpdate) -> ClockSync messages

client: GpuGeometryLibrary (bake every module into one set of 64-tree arenas, pack into single slabs)
        every frame: objects + clock -> PoseDriver -> SceneSnapshot (frames, instances, BVH) -> GpuSceneData
        render: PrimaryVisibility (compute) -> DirectLighting (fullscreen) in Bevy's Core3d graph
```

## Spec concepts in code

| Spec concept                                   | Code                                                                  |
| ---------------------------------------------- | --------------------------------------------------------------------- |
| Logical asset / canonical assembly             | `voxel::assembly::CanonicalAssembly`, `voxel::module::VoxelModule`    |
| Pose groups, motion domains, envelopes         | `PoseGroupTemplate`, `Motion`, `CanonicalAssembly::motion_envelopes`  |
| Deterministic animation from a shared driver   | `voxel::pose::PoseDriver`, replicated `ObjectAnimation` + world clock |
| Template-shared mini-BVH                       | `voxel::scene::AssetTemplate::mini_bvh`                               |
| Flattened pose-group leaves (Option A)         | `SceneAcceleration::FlattenedGroups`                                  |
| Resolved pose frame, instance-owned slot table | `ResolvedPoseFrame`, `SceneSnapshot::slot_frames`                     |
| Out-of-envelope fallback                       | `InstanceRecord::uses_template`                                       |
| `Node64`, leaf tag in `lod_meta`               | `voxel::node`, `lib/types.wesl`                                       |
| Two-`u32` masks and prefix counts              | `voxel::mask`, `lib/mask.wesl`                                        |
| `8 + 24` logical handles, typed arenas         | `voxel::handle`, `voxel::arena`                                       |
| No block crosses a page                        | `WordArena::alloc`                                                    |
| Adapter-aware single-slab residency            | `voxel::residency`, `client::render::device`, `GpuGeometryLibrary`    |
| Single-page root fast path                     | `Tree64Root::single_page`, `node_address` in `lib/tree64.wesl`        |
| 16-byte LOD payload                            | `voxel::lod`                                                          |
| Exact integer terminal-cell identity           | `VoxelHit::cell`, the visibility buffer's cell target                 |
| Visibility buffer (`R32G32Uint` + `R32Float`)  | `client::render::gpu::view`, `primary_visibility.wesl`                |
| `.vanta` files                                 | `voxel::format`                                                       |

## Correctness contracts and how they are enforced

- **Reference semantics.** `voxel::reference::check_hit` defines an acceptable hit: an occupied voxel
  the ray truly enters, at its entry distance, with nothing solid meaningfully closer. Ties and
  sub-ulp grazing contacts may resolve either way.
- **CPU traversal** is tested against brute force on boundary torture rays
  (`crates/voxel/tests/tree64_reference.rs`): node boundaries, corners, edges, boundary planes,
  near-parallel rays, inside starts, shared subtrees, multi-page layouts.
- **Scene search** is tested against exhaustive per-frame search for both acceleration modes over
  hundreds of animated, rotated instances (`crates/voxel/tests/scene_equivalence.rs`).
- **GPU** shaders are validated with naga and run on the real adapter against the CPU reference:
  masks bit-for-bit, 64-tree traversal by the reference rules on both page paths, and whole-scene
  traversal on the real assets (`crates/client/tests/gpu_conformance.rs`, `just test-gpu`).

Entry cells are chosen by comparing `t` against plane distances, never by flooring a reconstructed
position; flooring misplaced near-parallel rays by an ulp and missed geometry.

## Phase 1 status

Implemented: the data model and memory contracts above, the 64-tree bake and traversal, BVH4 scene
search in both organizations, the template-shared mini-BVH with per-instance refinement and
fallback, deterministic articulated animation (hinges and spins), the visibility buffer with exact
cell identity, direct sun/sky lighting with shadow rays, debug views (normals, pose frames, voxel
cells, traversal steps, group candidates, scene nodes, depth), and CPU and GPU benchmarks.

Not yet built (each a planned comparison, not a decision):

- The comparison hierarchies (8-way octree with bricks, sparse brick grid) and the packed-face
  rasterizer baseline. The old rasterizer was removed with the chunk engine and is in git history.
- Multi-slab residency: an arena larger than one binding reports `ExceedsSingleBinding`.
- LOD/footprint filtering (summaries are baked but unused), SGGX candidates, homogeneous media,
  transparent and refractive materials, temporal filtering, motion vectors, and GI.
- Traversal-state alternatives (stackless, bit-trail) and hardware ray queries.
- Non-rigid deformation (Tier 2) and flipbooks (Tier 3); editing and streaming.
- Camera-relative world origin for very large worlds.

Performance is the next priority. On an M1 Pro, the showcase scene traces 1280x720 primary rays in
roughly 90 ms flattened and 140 ms with template mini-BVHs (`gpu_scene_throughput`); grass and
foliage take about 170 DDA steps per pixel. Footprint LOD, traversal-state experiments, and
scene organization are the levers the spec lists.
