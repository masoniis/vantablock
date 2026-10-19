# Vantablock: Engine Architecture & Design Specification

**Version 3.1 - Updated Phase 1 Implementation Baseline**  
**Status:** see [`progress.md`](./progress.md) for what is built, measured, and next.
**Date:** October 18, 2026

> **Primary objective:** Build breathtaking, dense, close-up microvoxel worlds with intricate vegetation and geometry. Optimize around visual richness, animation, and simulation rather than maximum view distance.

**Priority order:** (1) Microvoxel detail; (2) extraordinary vegetation and geometry, including repeated complex structures; (3) animation, including authored animation; (4) physical simulation; (5) editable worlds; (6) far render distance.

**Design rule:** Canonical asset geometry, its articulated assembly, and animation/instance state are the single source of truth. Bounding volumes, traversal hierarchies, filtered LODs, cached transforms, visibility records, and lighting history are derived representations that can be regenerated or replaced. No optimization may silently change the geometry that should be visible.

**v3.1 implementation scope:** Preserve the pose-group spatial contract, deformation ownership, resolved visibility identity, adapter-aware memory fast paths, and leaf-tag ownership. Require exact terminal-cell spatial reconstruction and default to template-shared mini-BVHs. Add portable 64-bit mask arithmetic, traversal-state profiling, affine motion fast paths, optional SGGX-based foliage filtering, and homogeneous-medium skipping without freezing unmeasured optimizations. Preserve all visual goals and gates.

## 1. Core Technology Stack & Cross-Platform Strategy

### 1.1 Engine framework

- **Engine:** Full Bevy with ECS, `AssetServer`, windowing, input, audio, UI, and the `RenderApp` / `RenderGraph` infrastructure retained. Disable/replace only the relevant default 3D PBR pipeline, not the entire engine.
- **Graphics / compute:** `wgpu` with Vulkan on Windows/Linux and Metal on macOS. Probe adapter features and limits at startup; do not make experimental features a prerequisite for launching.
- **Portability target:** A functional compute-rendering path on both Vulkan and Metal. Optional accelerators must preserve the same scene semantics and images within stated tolerances.

### 1.2 Replaceable scene-level acceleration

An instance is a reusable piece of geometry placed in the world. A bounding volume lets a camera, shadow, or lighting ray skip irrelevant instances without examining their microvoxels.

- **Default, portable path:** Compute-based wide bounding-volume hierarchy (initial candidate: BVH4), using conservative axis-aligned bounds around transformed or animated instances. An oriented box may be tested precisely after identifying a candidate. The hierarchy may be refit for selected dynamic nodes and rebuilt when refitting quality deteriorates.
- **Optional hardware path:** Vulkan `wgpu` experimental ray queries / acceleration structures, activated only when available. Compare hardware-assisted candidate discovery against the software path; do not assume either wins. Verify support and current APIs against the locked wgpu version.
- **Common contract:** `query_scene_candidates(ray, t_min, t_max)` identifies candidate instances; `intersect_instance(ray, instance)` returns a world-ray hit or miss. Exact implementations may differ, but visibility, nearest-hit ordering, and material semantics must not.
- **Analytic entry/exit:** Candidate objects receive a local-space ray and an analytic ray-box intersection to find the interval within the object's conservative bounds. An acceleration structure's triangle hit should not be mistaken for an automatically available full box interval.

### 1.3 Required versus optional GPU features

- **Baseline:** Rely only on capabilities verified on the target Vulkan/Metal adapters. Keep optional subgroup operations, half-float arithmetic, hardware ray queries, and special texture formats behind feature checks or equivalent fallbacks.
- **Shader safety:** Standard checked WGSL during development. Any unchecked compilation path is a profiling-gated optimization after tests establish memory safety and real benefit.
- **Limits:** Query `max_storage_buffer_binding_size`, `max_buffer_size`, per-stage buffer bindings, and allocation limits. Never assume a single 1 GB storage-buffer binding exists on every supported adapter.

### 1.4 GPU memory addressing - adapter-aware fast path and pageable fallback

The engine distinguishes _logical_ geometry addresses from _resident GPU buffer locations_ without forcing page dispatch into the hottest voxel-traversal loop.

1. **Discover and request real supported limits.** At startup, query `adapter.limits()` and `adapter.features()`; select `required_limits` within the adapter's capabilities, and verify the created device's effective limits. When using Bevy, configure its wgpu settings so adapter-backed limits are actually requested (the appropriate priority/settings depend on the pinned Bevy version). `constrained_limits: None` by itself does **not** imply the full adapter limits were requested. Log `max_storage_buffer_binding_size`, `max_buffer_size`, storage-buffer binding count, available features, and per-arena allocations. The `128 MiB` wgpu default is not a universal hardware ceiling, **but 512 MiB or 1 GiB single bindings must not be assumed universally supported**.
2. **Single-buffer hot path, whenever legal.** Phase 1 begins with contiguous topology/LOD/attribute arenas that fit the _actual requested device_ limits and binding budget. Use direct arena indexing or a base offset resolved before entering the inner 64-tree DDA loop. Favor a single contiguous binding for a pose-group voxel root's reachable immutable topology where feasible; avoid page-slot `switch` operations at every node on this fast path.
3. **Logical paging for larger worlds and editing.** Retain a provisional 32-bit topology/attribute handle of `8-bit logical page ID + 24-bit 32-bit-word offset` (up to `64 MiB` addressable per page) and a resident page table mapping a logical page to a bound slab slot and within-slab offset. Reserve invalid/sentinel values; bit widths and page granularity are benchmark-gated. Different arenas are typed by the consuming operation, not silently conflated.
4. **Resolved-slot specialization.** If all references reachable within one immutable voxel root resolve to the same bound slab, resolve that slab **once at root entry** and use a compiled single-slab DDA variant. If a traversal must follow cross-slab references, take an explicitly correct multi-slab path with bounded/specialized dispatch, and quantify its cost. Workgroups or shader variants may specialize by residency class; no general promise that all assets fit into one slab.
5. **No _child block_ crosses a page boundary.** A contiguous sibling-header block (or leaf attribute block) must remain addressable within one logical page and in the intended arena; `base + popcount` increments by the actual element stride and must not wrap into another page. A logical asset, mutable edit path, or shared DAG **may** reference multiple pages/slabs. Absolutely prohibiting cross-slab assets would conflict with large assets, subtree reuse, and copy-on-write edits.
6. **Missing-page semantics.** Never treat nonresident geometry as empty without declaring that approximation. Use a conservative coarse proxy, deferred sample, or defined residency fallback; track page misses, fragmentation, available slots, and overflow. Budget topology, LOD, materials, pose frames, both BVHs, and page tables together, not in isolation.

**Gate:** Validate one-slab and multi-slab address decoding (including block boundaries) on the actual Vulkan and Metal adapters; measure per-ray page transitions, coherent vs. random reads, and whether page dispatch measurably degrades the inner DDA loop. Only then freeze the handle format and residency strategy.

## 2. Spatial Data Structures & Memory Layout

### 2.1 Separation of identity, animation, and spatial search

Vantablock explicitly distinguishes **four** concepts:

1. **Logical asset / canonical assembly:** A tree, crystal formation, building, or machine is one reusable logical asset containing canonical voxel modules and their attachment relationships. Logical asset identity does **not** imply one monolithic traversal tree.
2. **Pose groups:** Each rigidly articulated subgroup has a parent-relative transform, current/previous pose state, an immutable local voxel root, and a conservative bound enclosing its supported local deformation. A group may be a trunk, primary bough, machine rotor, or other movement domain. It is **not** necessarily a top-level Bevy entity.
3. **Spatial search structures:** Scene-level acceleration rejects unrelated logical objects. An optional _object-internal mini-BVH_ rejects pose-group bounds inside a composite asset. Finally, the pose group's own rigid local-space voxel hierarchy rejects empty microvoxel cells. These hierarchies need not match the _animation-parent_ tree.
4. **Continuous deformation domain:** Fine wind bending or procedural flutter within a pose group is handled by a separate intersection rule that knows the group identity **before** tracing through its initially empty interior. Groups are not chosen retroactively by the material/attachment ID of an as-yet-unfound voxel.

Thousands of moving leaves do **not** require thousands of scene-level BVH entries. However, independently rigidly transformed groups cannot simply be interleaved inside the cells of one undeformed `Node64` and treated as ordinary unchanged voxels. Each independently rigid group must be spatially resolvable and supply its own local voxel root or equivalent validated representation.

The system is **behavior- and material-driven, not species-driven**. A tree does not receive special bounding-box logic merely because it is vegetation; the same general articulation, deformation, opacity, and bounds contracts can serve vegetation, cables, cloth-like structures, machinery, and other assets.

### 2.2 Three-stage geometry search: scene / pose group / microvoxels

**Preferred composite-object path (Option B):**

```text
Unchanged world ray
    -> Scene BVH4 (one candidate logical-tree instance)
        -> Tree-local mini-BVH (candidate articulated pose-group bounds)
            -> Resolved pose-group frame (current + previous, per world instance)
                -> Inverse rigid transform into group-local coordinates
                    -> Group-local immutable 64-tree root (or alternative hierarchy)
                        -> Optional local fine-deformation intersection
```

- The tree's outer scene bound conservatively encloses all child-group poses and permitted fine deformation. In the tree-local mini-BVH, group bounds can be stored in tree-root coordinates; their current tight bounds can be refit or conservative envelopes can be reused. Candidate overlap is permitted, and nearest **world-ray** hits determine visibility.
- The internal BVH may contain tens of articulated groups, but **8-64 is a test workload, not a mandatory limit**. Tiny fluttering leaves need not become distinct pose groups if an unambiguous procedural field or a different validated representation handles them inside a larger group.
- Parent-relative rigid transforms allow motion without changing immutable microvoxels. A _resolved pose-group frame_ combines the particular world instance with its articulated local group, not just the shared asset's group template.
- **Alternative flattened path (Option A):** Register each articulated pose group directly as a scene BVH leaf. This avoids the second BVH but increases scene-leaf count and update pressure. Keep this implementation available for static, lightly articulated, or small-group-count scenes, and benchmark it against the internal BVH.
- **Unmoved groups:** A composite asset with no articulation may bypass the internal BVH entirely and trace its single local voxel root. There is no required two-BVH penalty for every object.
- **Default shared mini-BVH template:** Bake the object's internal mini-BVH **once per canonical articulated assembly**, using pose-group motion envelopes in asset-root space that conservatively include every allowed group state and descendant motion. World instances reference that immutable template; they do not need independent replicated/refitted mini-BVH trees solely because their poses differ.
- **Per-instance refinement:** After a shared template identifies a candidate group slot `g`, resolve that world instance's actual group frame and optionally test a tighter per-instance state bound before entering its voxel root. A contiguous `instance.base_frame_id + g` table is an optional fast path **only if** all instances use the same stable slot ordering and layout. Otherwise use an instance-to-group indirection or equivalent mapping; table compaction, missing groups, reparenting, and slot reuse must preserve history correctness.
- **Alternative for high-overlap poses:** A refitted per-instance mini-BVH or flattened scene-level pose groups remains available when shared worst-case motion envelopes incur excessive false-positive traversal. Any pose outside the baked motion envelope must trigger a conservative fallback, not silently disappear.

**Instance / pose-frame metadata (conceptual):** top-level asset ID, per-instance root transform, pose-group ID, resolved group-local voxel root, current/previous group-to-world transforms (or equivalent hierarchical source transforms), conservative bound, optional tighter-state bound, deformation-domain parameters, material override, residency version, and topology/history epoch. Store the minimum necessary fields; derive or cache resolved data after profiling.

**Bounds policy:**

- Precompute conservative envelopes for the specified animation domain, including maximum gusts. Every parent BVH must contain its descendents under every permitted state.
- Evaluate or cache tighter bounds where permanent envelopes would create too many false-positive tests; compare once-per-frame updates, demand evaluation, and hybrid cached refinement.
- The parent group's bounds may be reused by many descendants for animation state, but a shared wind driver does **not** imply identical per-group geometry movement.
- Instrument extreme overlap, invalid bound escape, refit cost, and number of candidate groups visited per ray. Geometry cannot be skipped merely because its _rest-pose_ voxels lie outside a straight world-ray interval.

### 2.3 Level 3: Pose-group-local microvoxel hierarchy

**Primary candidate:** A 64-way sparse hierarchy: each node addresses a `4 x 4 x 4` grid of cells. Five subdivisions cover a `1024 x 1024 x 1024` logical grid. This is a candidate, not an irreversible commitment.

**Mandatory comparison baselines:** An 8-way sparse voxel octree with appropriate bricks and a brick-based sparse grid, both tested on thin branching foliage and dense crystalline geometry.

**16-byte topology node (WGSL storage layout; proposed legal bit ownership):**

```wgsl
struct Node64 {
    child_mask: vec2<u32>, // 8 B: 64 occupancy bits
    child_ref:  u32,       // 4 B: FULL typed-by-context arena handle
    lod_meta:   u32,       // 4 B: bit31 leaf tag, low31 LOD index
};                            // 16 B; 8 B alignment
```

- An **internal** node's `child_ref` points to the first of its contiguous occupied-child `Node64` headers; a **terminal** node's `child_ref` points to a contiguous leaf-attribute block. The type is determined by `lod_meta & 0x80000000u`; it is **not also hidden in `child_ref`**.
- This resolves v3.0's bit collision: the proposed `8-bit page ID + 24-bit word offset` consumes every bit of `child_ref`. The remaining `lod_meta` low 31 bits are an LOD index/handle with an explicitly bounded format; LOD pages cannot secretly assume an extra bit. `0x7FFFFFFF` may be reserved for missing LOD if the allocator declares it.
- Child `k` is addressed as `base_word_offset + 4 * popcount64_prefix(mask,k)` because `Node64` occupies four 32-bit words. Parent/sibling blocks may not straddle a logical page boundary. Attribute blocks obey their own element stride.
- If the device uses the single-slab fast path, the page/slot calculation can be hoisted from the inner loop; a multi-slab path is still required whenever links span residency slabs. Exact handle encoding is **provisional**, validated by unit tests and real WGSL, not a permanent API promise.
- Only _rigid pose-group-local_ canonical voxel cells belong in this hierarchy. Applying one pose-group transform to all its voxels is valid; inserting independently rotated other groups into the same static spatial cells without a secondary search structure is not.
- Traversal uses exact cell boundaries for opaque hits. Leaf/LOD traversal may employ separately validated aggregate filtering.

**Portable 64-bit mask prefix count (WGSL baseline):** Portable WGSL uses two `u32` mask halves rather than assuming native `u64`. For zero-based `k` in `0..63`, the count excludes child `k`. Multiply by four **32-bit words** per `Node64` when advancing a topology `child_ref` within its contiguous child block.

```wgsl
fn child_word_offset(mask: vec2<u32>, k: u32) -> u32 {
    let high = k >= 32u;
    let below = (1u << (k & 31u)) - 1u;
    let lower = countOneBits(select(mask.x & below, mask.x, high));
    let upper = countOneBits(select(0u, mask.y & below, high));
    return (lower + upper) * 4u;
}
```

- **Correctness:** Unit-test all `k = 0..63` with empty/full masks, both half-boundaries (`31/32`), isolated bits, randomized masks, and CPU bitset reference. Require only occupied child indices when dereferencing. Also test page-end sibling blocks and logical-handle decoding separately; this function offsets within one block, **not** across pages.
- **Performance:** Treat branchless `select` as an initial candidate, not a guarantee of no divergence or superior performance. Compare generated GPU execution and a readable conditional variant on reference adapters; optimize the actual measured inner loop.

### 2.4 Topology, LOD, and material separation

1. **GlobalTopologyArena - hot:** 16-byte pose-group-local nodes and compact contiguous child blocks, fetched during hierarchical traversal. A local root may share immutable descendant data with other instances and groups.
2. **GlobalLodArena - warm:** Aggregate geometry and shading data, usually accessed only when ray footprint or filtering requires it.
3. **GlobalAttributeArena - cold:** Palette references and per-microvoxel material overrides, fetched after actual leaf hits.

**Proposed 16-byte LOD payload (four `u32` words):**

| Word | Proposed packed contents                                      |
| ---- | ------------------------------------------------------------- |
| 0    | `f16` solid volume fraction + `f16` axis-X projected coverage |
| 1    | `f16` axis-Y coverage + `f16` axis-Z coverage                 |
| 2    | Packed normal-distribution / anisotropy approximation         |
| 3    | Representative material or palette-mixture descriptor         |

Packed half values can be stored as bitfields in `u32`; this does not require native `f16` storage fields. Exact packing, roughness/orientation fidelity, and material-mixture statistics remain visual-quality experiments. **Axis coverage is an approximation, not mathematically exact directional opacity.**

**SGGX-based directional candidate (not a mandated replacement):** Benchmark a symmetric positive-semidefinite projected-area tensor `S` with six independent coefficients. Its direction-dependent projected-area statistic is `sigma(w) = sqrt(max(0, w^T S w))`, where `w` is a unit local ray direction. The tensor may also aid anisotropic microflake shading. Six `f16` coefficients occupy 12 bytes before volume, density, palette, or other data; retain an explicitly budgeted separate/additional attribute representation if required rather than silently pretending all semantics still fit in 16 bytes.

- **Do not equate projected area with alpha:** SGGX projected microflake area is not, by itself, exact silhouette coverage, opacity, extinction, or transmittance of overlapping finite voxels. Those require an independently validated density/optical mapping, especially for multiple foliage layers.
- **Evaluation:** Bake the current axis-coverage proxy, SGGX candidate, and at least one higher-quality reference representation from the same canonical assets. Compare angular coverage, transmittance, silhouette/shadow appearance, temporal stability, and cost against offline supersampled ground truth for thin diagonal leaves, dense canopies, and faceted crystals. Pick the smallest acceptable representation by material/workload; no general `sqrt(3)` worst-case claim is assumed.

### 2.5 Immutable deduplication and bounded mutability

- **Cross-instance reuse:** Reuse the same geometry asset for differently placed, rotated, scaled, tinted, or procedurally animated instances.
- **Within-library reuse:** Offline asset baking hashes immutable leaf bricks, exact child blocks, and shareable descendant structures. Verify full equality after hash matches; hashes alone are not proof of identical geometry.
- **Important restriction:** A contiguous-children layout does _not_ support arbitrary sharing of all subtree roots. It can share identical child blocks and descendants referenced by copied headers; compression metrics must describe this actual capability rather than assuming unrestricted DAG deduplication.
- **Pose-group sharing:** The _template_ voxel module and its immutable local root may be shared across trees, while the resolved pose-group frame, world-instance binding, and animation state differ. Do not confuse a shared root address with a unique runtime pose-frame identity.
- **Variation:** Prefer sharing reusable structures (leaf models, branch modules, crystal motifs) while changing assembly, size, seeded growth parameters, orientation, palettes, and movement. A forest need not consist of identical whole-tree assets.
- **Copy-on-write editing:** On a local edit, duplicate the necessary contiguous sibling-header blocks along the modified root-to-leaf path; leave unchanged descendant references pointing to immutable geometry. Budget up to about `5 * 64 * 16 = 5 KiB` of copied _headers_ for a five-level path, plus any leaf attributes, LOD data, allocator overhead, and fragmentation. Avoid implying 5 KiB is the total worst-case edit cost.
- **LOD updates after editing:** Recompute affected aggregates with an approximation or localized rebake. Exact projected coverage cannot generally be recovered from only each child's three coverage scalars, because projected silhouettes may overlap.
- **Lifetime management:** Plan page reclamation, safe GPU synchronization, and sharing/reference lifetime before promising bounded memory growth.

### 2.6 Content generation and `.vanta` assets

A dedicated `vanta_baker` produces both geometrically beautiful assets and efficient render representations.

- Parametric space-colonization and L-system tree/vegetation generation; controllable branch growth, leaves, vines, bark microstructure, and roots.
- Crystal generation using procedural growth, Voronoi-like structure, SDFs, repetition, and carefully authored motifs.
- GPU/CPU mesh and SDF voxelization for Blender/glTF assets, with material preservation.
- Per-asset canonical voxel modules, explicit attachment/pose-group assembly, pose-group-local spatial roots, an optional **template-shared internal mini-BVH** baked from conservative group envelopes, filtered LOD statistics, material palettes, and deduplication artifacts.
- Hot reload / rapid Bevy preview. Render the same asset with several traversal backends without regenerating its canonical geometry.
- Provide a high-quality offline/supersampled reference view to evaluate approximations rather than judging fidelity solely by realtime screenshots.
- **Certified homogeneous-medium summaries:** For each node/brick, optionally bake a recursive `uniform_filled_medium` (or equivalent) certificate only when its complete occupied volume contains no cavities or internal optical-property transitions that matter for the selected refractive model. Do not infer this from a full 64-bit occupancy mask alone. Preserve medium ID, refractive parameters, and attenuation data, with explicit invalidation after edits; these summaries are derived from canonical geometry.

## 3. Bevy RenderGraph & Shading Pipeline

### 3.1 Modular render stages

| RenderGraph node         | Role                                                                                                                                                                | Phase / qualification    |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------ |
| `VoxelScenePrepNode`     | Resolve active pose-group frames, update residency and pose bounds, prepare optional object-internal mini-BVHs plus scene BVH / hardware TLAS                       | Base                     |
| `PrimaryVisibilityNode`  | One unchanged world-space ray; scene candidates -> pose-group candidates -> local voxel intersection; output resolved pose-frame ID, geometry hit, depth and motion | Base                     |
| `DirectLightingNode`     | Material lookup, direct sun/sky lighting, shadow intersections, basic atmosphere                                                                                    | Visual showcase          |
| `IndirectAndRefractNode` | Optional coarse GI, irradiance cache or ReSTIR-like techniques, crystal/glass/water transmission                                                                    | Deferred until justified |
| `TemporalFilterNode`     | History validation, denoising, antialiasing and optional upscale                                                                                                    | Incrementally improved   |

No advanced GI method is mandatory for the Phase 2 static showcase. Simple lighting that looks excellent and runs efficiently is preferable to complicated GI with unstable artifacts.

### 3.2 Single world ray, group-owned intersection and deformation

**Non-negotiable traversal invariant:** A camera, shadow, or GI ray follows its ordinary straight world-space path until a physical optical interaction changes it. Animation never refracts or permanently bends that world ray.

1. Traverse the scene hierarchy with the unchanged world ray and find a logical-object candidate within its conservative world bound.
2. If the logical asset is articulated, traverse its optional **object-local mini-BVH** to enumerate conservative pose-group candidates. The alternate flattened path enumerates those groups directly from the scene BVH. Both paths yield the same _resolved pose-group frame_ and conservative world-ray interval.
3. For a rigid group, inverse-transform the ray into that group's rigid local frame. Affine transforms preserve straight lines, allowing ordinary group-local voxel DDA against its immutable root.
4. For a fine-deformable group, use a specialized local intersection method, but only when its spatial deformation is defined **before any geometry hit**. A spatial inverse `D^{-1}(p)` must be single-valued within the chosen traversal domain and sufficiently regular for the selected intersection solver. The possibility of folding, reentry, disconnected swept geometry, or approximation error must be handled or routed to a different method. Conservative swept bounds alone are necessary but do not by themselves prove that a segmented curved-ray solver cannot miss a hit.
5. Test all overlapping candidate groups independently as necessary. Convert local hits to `t_hit` along the **original world ray**, compare in the same world-distance ordering, and preserve their resolved pose-group identity. Continue the unchanged world ray after rejected candidates.

**Empty-space identity rule:** A ray has no voxel/material/attachment ID until it finds geometry. A shader cannot pick an inverse wind deformation using the identity of a voxel that has not been intersected. If independently moving branches overlap, they require separately identifiable bounded domains (e.g. pose-group roots), a unified _well-defined_ spatial field, or a different validated intersection algorithm; post-hit material IDs may affect shading but cannot retroactively steer an unhit ray.

**Optimization principle:** Group-local canonical geometry remains immutable during ordinary animation. Parent-relative transforms operate on pose-frame metadata, not on millions of individual voxel positions. Spatial-search groupings and animation parentage can differ, but the group whose voxel root is intersected must be identified before using its local deformation rule.

**Traversal-state/register-pressure guardrail:** A combined scene-BVH, pose-group mini-BVH, and 64-tree shader may carry multiple candidate stacks, intervals, and coordinate frames. This can reduce occupancy or cause spills on particular GPUs, but no specific register count is assumed without compiler/profiler evidence.

- Baseline correctness implementation: bounded private traversal stacks/state and transient local ray transforms for each candidate; preserve world-ray nearest-hit ordering and overlapping candidates. Measure performance before replacing working algorithms.
- Compare a short private stack, a proved-correct stackless/restart or bit-trail strategy, and a hybrid for the shallow voxel hierarchy. Stackless restart can re-visit nodes; a `var<workgroup>` stack is **shared memory**, not free private registers, and may hurt occupancy or require per-lane indexing. Do not promise that either is faster.
- Track compiler register allocation, spills/local-memory traffic, shared-memory occupancy, wave utilization, stack depth, repeated node visits, and GPU time on Vulkan and Metal. Specialize kernels or split stages only when this reduces total frame cost including intermediate-buffer traffic.

### 3.3 Visibility and depth buffers

**Proposed `R32G32Uint` opaque-hit identity plus separate `R32Float` world-ray distance:**

- `vis.x`: **20-bit resolved pose-frame ID** (the unique active _world instance + articulated local group_, not the top-level Bevy instance ID), **3-bit LOD**, **6-bit terminal cell index**, and **3-bit hit/face code**.
- `vis.y`: 32-bit logical voxel node or leaf-block handle; the handle identifies _data_, not a unique spatial placement when DAG subtrees are reused. The separate spatial-occurrence record/encoding provides the missing integer location.
- `depth`: world-ray hit parameter `t_hit` under a documented ray-normalization convention; non-hit sentinel and precision tests are required.
- **ResolvedPoseFrameTable:** Each pose-frame ID resolves to the parent asset/scene instance, immutable voxel root, current and previous group-to-world frames, deformation domain and current/previous parameters, material overrides, topology/residency epochs, and related references. Reuse of pose templates does not imply reuse of runtime pose-frame IDs. Resolve template pose-group slot to active frame using direct base+slot indexing only when slots are contiguous and stable; otherwise use instance-owned indirection.
- **Reconstruction and exact location:** Recover the continuous world-space hit from the unchanged world ray and `t_hit`, then transform it into the resolved group frame for shading. **Do not determine the exact hit voxel by `floor()` of this floating-point reconstructed position alone.** Traversal must preserve enough _integer spatial-occurrence information_ to identify the terminal voxel or LOD cell unambiguously, even if a DAG node is shared at multiple lattice positions. The existing 6-bit terminal cell index identifies a position **only within its `4 x 4 x 4` parent**, not the parent location.
- **Boundary tie-break:** A half-voxel inward normal bias or replacement of reconstructed low coordinate bits may help under restricted conditions, but neither is a general correctness proof. For example, if the correct `X = 64` rounds below the boundary to `63`, overwriting the low two bits of `63` with `00` yields `60`, not `64`. Encode traversal-derived brick/ancestor coordinates, a uniquely resolvable spatial occurrence, or a validated equivalent before relying on that optimization.
- **Candidate spatial encoding:** For the initial `1024^3` _pose-group-local_ grid, three 10-bit integer cell coordinates fit in 30 bits. A separately encoded 32-bit local-cell-coordinate attachment is a correctness-first baseline; LOD level and the aligned cell/brick corner define coarser hits. Other compact encodings may replace it after GPU bandwidth and reconstruction tests. A full 32-bit additional per-pixel attachment costs about `14.06 MiB` (`14.75 MB`) at `2560 x 1440`, excluding auxiliary reads/writes. Do not assume the 64-bit visibility buffer already has room for an extra full coordinate.
- **Correctness tests:** Compare exact terminal-cell/LOD-region identity against a CPU or validated reference for boundary coordinates (4, 16, 64, 256, 1024), grazing rays, axis ties, both ray directions, nonuniform transforms, large world offsets, neighboring empty voxels, shared DAG occurrences, and current/previous animated pose frames. Make pass/fail independent of temporal filtering.
- **History:** The 3-bit hit/face code cannot simultaneously store an arbitrary normal, topology-change flag, and group-generation stamp. Track temporal validity using stable pose-frame identity plus separate generation/epoch information or a dedicated history-validity output. Handle recycled table slots and pose reparenting explicitly. Nonrigid motion vectors also require current/previous forward deformation evaluation for the found surface point.
- **Capacity:** 20 bits address at most `2^20 = 1,048,576` simultaneously active pose frames, not necessarily the same number of top-level objects. Specify overflow behavior and profile table bandwidth. Transparent/multilayer hits use a separate appropriate traversal/record path.

### 3.4 Fine-geometry filtering: a measurable approximation

For ray-cone footprint `r(t)`, deeper traversal is skipped when a region is smaller than the footprint **and** an acceptable filtered representation is available. Coverage and material averages are precomputed offline.

- Initial LOD filtering uses solid volume fraction, a compact set of projected axis coverages, a normal/roughness approximation, and representative material statistics.
- Treat `alpha(direction) ~ clamp(sum(abs(direction_axis) * axis_coverage), 0, 1)` only as a **low-cost directional proxy**. It is neither exact arbitrary-angle coverage nor guaranteed accurate for overlapping thin geometry.
- Benchmark richer directional bases, limited sampled directions, and stochastic transmission if the simple proxy fails on overlapping leaves or faceted crystals.
- Use spatially/temporally stable stochastic choices, validated history rejection, and scene-ground-truth comparisons to avoid flicker, blur, and foliage disappearing at distance.
- Optimize by projected _visual impact_, not only camera distance. Large silhouettes and moving shadows may need detail even far away; imperceptible leaf flutter may be simplified close up.

### 3.5 Lighting and transparent materials

- Opaque primary visibility uses the compact nearest-hit visibility buffer.
- **Transparent solids need a distinct traversal mode:** Track entry/exit boundaries and material transitions, not every artificial internal interface between adjacent voxels of the same crystal or glass material.
- **Homogeneous-interior fast path:** When `vanta_baker` certifies a fully filled region with one physically equivalent optical medium, the refractive traversal may analytically advance from the entry point to that region's box exit, skipping internal microvoxel steps. Crossing a **node boundary is not automatically crossing a material boundary**: continue through adjacent same-medium regions without refraction and refract/reflect only at actual medium transitions.
- **Correctness conditions:** Apply any Beer-Lambert absorption/transmittance over the traveled world-space distance; preserve total internal reflection, interface normals, and path lengths. Internal cavities, differing IOR/absorption, editing, and refractive heterogeneity invalidate the homogeneous certificate or force finer traversal. Test against unsimplified optical traversal before enabling the optimization.
- Water uses its own reconstructed surface representation with refraction/reflection rays querying the same scene geometry.
- Secondary rays may use coarser geometry and simpler illumination, but must obey conservative shadowing and depth semantics.
- SGGX-style microflake shading, ReSTIR GI, and sophisticated denoisers are optional candidates, not architecture-wide dependencies.

## 4. Animation, Simulation & World Mutability in Bevy

### 4.1 Universal three-tier animation architecture

Animation is chosen by **movement behavior**, not by asset category.

**Tier 1 - Hierarchical rigid / articulated pose groups**

- A logical asset's canonical assembly contains parent-relative rigid pose groups. Every independently rigidly transformed group has a **separately identifiable conservative bound and group-local voxel root**, but does not have to be a top-level scene BVH entry.
- Default composite objects are resolved through `scene BVH -> optional object mini-BVH -> group-local voxel hierarchy`; a flattened scene-BVH-per-pose-group view remains an equivalent benchmarked alternative.
- The _animation parent tree_ determines inherited transforms, but the _object-local mini-BVH_ is built for spatial culling. Do not insert group-transform operations between arbitrary `Node64` occupancy steps as if independent moved groups still resided at their original grid coordinates.
- Transform candidate rays into the rigid group's current local frame. Store or reconstruct the corresponding previous frame for motion vectors, and keep stable runtime resolved pose IDs for history validation.
- Reused geometry modules retain one immutable voxel root but may occupy many world instance/group frames with distinct transforms, materials, wind phases, and seeds. A template-shared object mini-BVH culls conservative motion envelopes across all instances; optional tighter per-instance state bounds are derived and cannot exclude legal poses.

**Tier 1.5 - Affine and piecewise-rigid motion fast paths (benchmark candidates)**

- Prioritize complete-group affine maps (translation, rotation, scale, invertible shear) where the visual result is acceptable. Transform rays by the inverse affine map; straight rays remain straight, so the existing group-local voxel DDA runs unchanged. Transform normals correctly (inverse-transpose where applicable), handle mirrored/nonuniform scales and world-ray `t_hit` consistently, and conservatively update/envelope each group's bounds.
- Model stronger curvature with a small number of independently bounded rigid/affine segments _only where warranted_. Piecewise motion may need extra pose groups and local traversal, and can introduce seams or articulation costs. It is not automatically visually equivalent to smooth bending.
- Do **not** prohibit nonlinear deformation above the terminal `4x4x4` level. A large flexible branch may require smooth bending; restrict such methods by demonstrated intersection correctness and performance, not an arbitrary leaf-only rule.

**Tier 2 - Predictable continuous deformation within a known domain**

- Global wind supplies a shared driving field; geometry groups supply their own stiffness, phase, bend response, and optional locally seeded variation. Static geometry has no deformation cost.
- **Pre-hit causality:** A group-local inverse map must be well-defined in empty space _without first knowing which voxel will be hit_. Multiple independently moving, overlapping attachment domains cannot be selected by a post-hit voxel ID. Either split into distinct pose-group envelopes with independently tested roots; define one genuine single-valued invertible spatial field on the whole chosen group; or use forward-updated bounds/geometry or another explicit intersection approach.
- Conservative motion envelopes protect broad phase. For nonlinear bending, benchmark validated segmented inverse paths, tighter state-dependent bounds, and occasional forward-updated local geometry. An inverse map's existence does not by itself guarantee efficient or miss-free curved-path DDA.
- Avoid computing individual leaf flutter before candidate culling whenever possible. A deterministic hash/seed can produce distinct leaf motion without maintaining one animation timeline per leaf, **provided** the corresponding traversal method has a defined deformation domain and conservatively encloses its results.

**Tier 3 - Precomputed voxel animation sequences**

- Flipbook clips may select among immutable deduplicated voxel roots or changed subtree segments when motion is not well described by rigid transformations or a reversible local field.
- Changing topology or the surface correspondence invalidates or reduces temporal history as appropriate. Avoid treating new voxel surfaces as rigidly corresponding points.

### 4.2 Optional animation level of detail

- Near or highly visible: individual leaf flutter, small-branch bend, fine articulated motion.
- Intermediate: coherent branch/group motion with reduced fine details.
- Far or visually insignificant: coarse canopy silhouette movement, or static filtered appearance when motion has negligible projected effect.
- **Removable module:** Animation LOD must be feature-gated and benchmarked. It may never alter canonical geometry or prevent full-fidelity rendering. Transitions need hysteresis and stable temporal reprojection.

### 4.3 Shared-input, distinct-response animation bounds

Three testable policies should coexist behind a common interface:

| Policy                                                             | Cost advantage                    | Risk                                   |
| ------------------------------------------------------------------ | --------------------------------- | -------------------------------------- |
| Update tight pose-dependent bounds once/frame                      | Reuse across many rays            | Per-frame updates and hierarchy refits |
| Compute tighter bounds only upon ray visit                         | Skip unseen regions               | Repeat the same math across many rays  |
| Conservative outer bound + group-level cached/current inner bounds | Balance update and traversal cost | Cache, state/version synchronization   |

Provisional preference: **template-shared conservative outer/group mini-BVH + selective per-instance group-level tightness**, especially for correlated wind. A per-instance refitted mini-BVH remains benchmarkable where maximum-motion envelopes are too loose. Do not commit until workloads show the benefit. Bound conservativeness and excessive candidate visits are first-class benchmarks.

### 4.4 Simulation remains decoupled from voxel storage

- **Dust, sparks, granular particles:** GPU simulation with specialized point/box/batched rendering; microvoxels where visually useful.
- **Rigid debris / fractured pieces:** Physics driven by transforms; share existing voxel sub-assets where possible.
- **Fluids:** PBF/FLIP, shallow water, or a simpler validated solver as appropriate. Render water with screen-space/surface reconstruction or local implicit surface methods rather than force every particle into a voxel tree.
- **Coupling:** Coordinate collisions, lighting, shadows, and surface interaction through shared world interfaces; do not require identical internal geometry storage for different media.

### 4.5 Editing and destruction (lower priority)

- Treat immutable geometry as the default fast path. Edits allocate copy-on-write child blocks and replacement attributes in separately managed mutable slabs; modified groups may reference shared immutable descendants across slabs, triggering the correct multi-slab access path if necessary.
- Preserve unchanged shared descendant subtrees.
- Update affected derived bounds, LOD summaries, and ray visibility history without globally rebuilding every instance.
- Bound memory growth by lifecycle policies; deformation and edit paths must coexist without silently modifying shared canonical assets.
- More sophisticated structural destruction and stress solvers can integrate later without making physics the foundation of the renderer.

## 5. Phased Execution Roadmap & Gate Criteria

**Schedule:** Approximate research milestones, not delivery commitments. Each phase ends with an experiment and a go/no-go decision. The same canonical test assets must work across competing traversal pipelines.

### Phase 1: Renderer Feasibility & Memory Contracts (Weeks 1-4)

**Deliverable:** Build `vanta_baker` for a demanding `512^3` detailed branching/crystal asset and implement these paths on the same scene: (1) software BVH4 + 64-tree; (2) software BVH4 + one alternative hierarchy (8-way octree with bricks or sparse brick grid); (3) optional Vulkan hardware-assisted scene broad phase if supported.

**Tests:** Close inspection and hundreds of overlapping, rotated instances; sparse branches and dense crystals; still and moving camera. Measure GPU frame passes, rays/sec, false-positive bounds, active VRAM, coherence, subpixel stability, and cross-backend behavior. Validate the actual 16-byte node packing, leaf tag/handle capacity, child-block page boundaries, and both single-slab and cross-slab binding schemes on Vulkan and Metal. Include a minimal **two-group articulated asset** with one shared voxel module under two differently rotating transforms; verify identical hits from flattened versus internal mini-BVH traversal. Add a **boundary torture test** that compares exact integer terminal cell/LOD region to a CPU/reference intersection, especially at brick edges and shared DAG occurrences. Prototype **one template-shared mini-BVH across hundreds of varied pose instances** and compare memory, candidate visits, and total GPU time to per-instance refitting and flattening.

**Additional Phase 1 tests:** Validate the WGSL two-word occupancy/prefix function against CPU references for all 64 child indices and page-edge allocations. Profile the combined traversal shader's registers, spills, occupancy, stack depth, and repeated visits; compare a short private stack with at least one alternative only after the baseline is correct. Capture Vulkan and Metal differences and total GPU time, not only isolated inner-loop throughput.

**Gate:** At least one hierarchical path produces convincing detail with acceptable performance and no missed geometry; a rotated pose group stays hittable, and visibility reconstructs **exact terminal-cell identity**, its world position, and motion correctly at boundaries and across deduplicated geometry. Template-shared mini-BVH traversal must be correct for all legal pose states; it is not required to win every benchmark. **Do not lock the 64-tree, internal BVH, visibility packing, paging layout, or TLAS simply because the proposal favors them.**

### Phase 2: Static Visual Masterpiece (Weeks 5-10)

**Deliverable:** A visually complete, dense jungle-to-crystal-cavern scene with authored/procedural microstructure, material palettes, real close-up scale, baseline direct sun/sky lighting, atmosphere, anti-aliasing, and robust foliage filtering.

**Target (aspirational):** 1440p / 60 FPS within a 4 GB VRAM budget on an explicitly named reference GPU. Keep a lower-tier fallback and report scene-dependent bottlenecks. Test slow pans, close approaches, overlapping leaves, thin crystal facets, and memory occupancy; compare to offline/supersampled images.

**Additional Phase 2 test:** Bake both axis-coverage and SGGX-style directional LOD candidates from identical voxel sources; compare angular opacity, foliage silhouettes, shadows, and motion against offline supersampled reference images and measure LOD bytes, shader traffic, and frame time. Keep the axis proxy if it meets quality targets.

**Reference hardware:** The reference GPU for the target is the RTX 3080 (Vulkan, hardware broad phase), with the M1 Pro (Metal, software broad phase) as the lower-tier fallback. Check register pressure with `just shader-stats` as traversal grows (128 registers per thread on NVIDIA for the software path, 80 with ray queries).

**Gate:** The scene must be visually impressive _before_ adding ReSTIR GI or advanced simulation. Reject approaches that obtain frame rate by visibly deleting or blurring fine detail.

### Phase 3: Living Scene & Animation (Weeks 11-15)

**Deliverable:** Full parent-relative articulated pose groups and object-internal mini-BVH versus flattened-group acceleration benchmarks; shared-input/per-domain-response wind; a _validated_ pre-hit deformation field or alternative local intersection; conservative bounds; deterministic procedural leaf variation; current/previous pose reprojection and stable identity/epoch tracking. Optional animation LOD prototype.

**Tests:** Hundreds to thousands of articulated foliage groups, both correlated and opposing responses; gust extremes; static rocks interleaved with independently bending branches; overlapping rest/deformed domains; folded/noninvertible stress cases routed to the fallback; rapidly changing cameras. Compare **template-shared conservative mini-BVH plus per-instance tight bounds**, separately refitted per-instance mini-BVHs, and flattened group leaves; also compare on-demand, frame-updated, and hybrid bounds. Include group-slot remapping and animation outside a baked envelope, with an explicit safe fallback. Include the hardware broad phase in the organization comparison at thousands of groups, and compare rebuilding its TLAS each frame against refitting it.

**Additional Phase 3 tests:** Compare hierarchical rigid transforms, whole-group affine shear, piecewise-rigid segments, and verified nonlinear deformation on thin branches, gusts, and changing silhouettes. Measure quality/seams, exact hit agreement against a reference, false-positive bounds, motion-vector validity, and total animation-plus-traversal cost.

**Gate:** No missed geometry under valid animation limits; controlled ghosting; measurable savings compared to rebuilding voxel occupancy each frame. Use a separately measured animation overhead budget rather than guaranteeing an unverified <1 ms result.

### Phase 4: Lighting Refinement & Refractive Crystals (Weeks 16-20)

**Deliverable:** Improve GI, temporal filtering, atmosphere, material transmission, and distinct crystal entry/exit handling. Compare coarse secondary-ray LOD, irradiance caching, and an optional ReSTIR-style solution. Secondary rays can share the hardware broad phase's TLAS on capable adapters.

**Additional Phase 4 test:** Validate baked homogeneous-medium certificates against canonical voxel occupancy and materials, then compare optical traversal with and without homogeneous interior skips. Match real interface locations, world-space path lengths, absorption, total internal reflection, and transmitted color, including edited/cracked crystals.

**Gate:** Clear visible quality improvement at an acceptable incremental cost; crystalline surfaces must not refract at artificial internal voxel boundaries.

### Phase 5: Physical Simulation (Weeks 21-26)

**Deliverable:** Granular particle effects, instanced debris, and a fluid demonstration integrated with voxel lighting and collisions. Keep fluid surface representation independent from opaque microvoxel occupancy.

**Gate:** Interactive scene at practical target performance with visually stable water/debris. Simulation can run at coarser spatial and temporal resolutions than rendering.

### Phase 6: Localized Mutability (Weeks 27-31)

**Deliverable:** Mutable slab management, child-block copy-on-write edits, local LOD/bounds repair, and lifetime reclamation. Prototype carving, structural removal, and simple destruction.

**Gate:** Local edits avoid globally duplicating assets or stalling the renderer; memory use and editing cost are measured rather than assumed constant.

### Phase 7: World Streaming & Distant Geometry (Post-validation)

**Deliverable:** Out-of-core asset paging, coarse horizon representation, clipmap/streaming experiments, and expanded world size.

**Gate:** Never sacrifice near-field microvoxel quality, remarkable vegetation, or animation merely to increase viewing distance.

## 6. Architecture Decisions, Replaceable Interfaces & Open Questions

### 6.1 Decisions to preserve

1. Beauty and microvoxel fidelity outrank view distance, arbitrary editing, and engine feature count.
2. The scene uses one coherent world-ray traversal; local animation changes the _intersection test_, not the trajectory of unrelated rays.
3. One logical asset may contain many pose-group-local voxel roots; independently rigid groups are independently _spatially addressable_ but need not become independent top-level scene instances.
4. Parent-relative rigid transformations are preferred wherever sufficiently convincing; fine nonlinear deformation is selective.
5. Shared environmental drivers (such as wind) produce different per-object/per-subregion responses; there is no universal spatial warp imposed on static geometry.
6. Conservative motion envelopes guarantee no missed intersections, with tighter state-aware bounds only where they improve cost.
7. Logical asset assembly, animation parent hierarchy, pose-group intersection domains, and spatial BVHs are deliberately distinct; rigid group transforms must not be inserted into an otherwise static `Node64` occupancy grid.
8. Canonical geometry and animation state are authoritative; bounds, caches, compression, LOD, and shading representations are derived.
9. Foliage filtering, GI methods, hardware RT, animation LOD, and detailed mutability are replaceable experiments, not foundational assumptions.
10. The identity returned by visibility is a **resolved pose-frame ID**, not just a scene-instance index; immutable DAG node references do not uniquely identify spatial placement. Exact integer terminal-cell or LOD-region identity must survive traversal, independent of floating-point boundary rounding.
11. GPU storage prioritizes adapter-legal contiguous fast paths, with correct cross-slab references when large assets, mutable edits, or residency demand them; no mandatory per-step page dispatch for the fast path.
12. A 32-bit handle cannot simultaneously contain a full 8+24 page/word address and an additional type bit; the provisional tag lives in `lod_meta` instead.
13. Diverse procedural assembly and reusable substructure are both valuable; unique-looking trees matter more than maximizing whole-tree instancing counts.
14. Object-internal mini-BVHs default to **canonical template sharing** with conservative pose envelopes; per-instance tight bounds and dynamically refitted/flattened search structures are derived, benchmarked alternatives. Group-slot addressing must not assume every instance has an identical contiguous pose table.
15. Portable two-`u32` occupancy masks and prefix population counts require exhaustive correctness tests; branchlessness and stackless traversal are optimization candidates, not absolutes.
16. Prefer validated affine/rigid deformation fast paths when they look convincing, without prohibiting correct nonlinear deformation needed for intricate organic animation.
17. SGGX-style directional data and certified homogeneous optical regions are optional derived summaries. Neither may silently replace exact geometry or omit physically meaningful material transitions.

### 6.2 Interfaces to keep swappable

```text
VantaAsset / CanonicalVoxelModules + CanonicalAssembly
  -> BakeGeometryRepresentation(asset, voxel_layout)
       -> ImmutableVoxelRoots + LeafAttributes + LODSummaries
       -> OptionalSGGXDirectionalSummaries + CertifiedHomogeneousMedia
       -> OptionalTemplateSharedPoseGroupMiniBVH + ConservativeEnvelopes

WorldInstance + LocalPoseGraph + AnimationModel
  -> ResolvedPoseFrameTable(world_instance, local_group)
  -> ConservativeBounds(instance, group, time_range)
  -> OptionalTightBounds(resolved_frame, current_state)
  -> ResolveTemplateGroupSlot(instance, group_slot) -> resolved_pose_frame

SceneCandidateQuery(world_ray)
  -> logical-object candidates (or flattened pose-frame leaves)
PoseGroupCandidateQuery(instance, world_ray, interval)
  -> resolved pose-frame candidates + conservative world intervals

PoseGroupIntersection(world_ray, resolved_frame, interval, mode)
  -> nearest world-ray t_hit + local voxel handle + group ID
     + exact integer terminal-cell/LOD-region spatial identity
     + material/LOD identity + motion-reconstruction data

LightingQuery(world_ray, mode, lod_policy)
  -> opaque hit / transmitted segments / shadow response
```

These are **conceptual contracts**, not literal Rust or WGSL APIs. Implement specialized shader variants rather than paying virtual dispatch per microvoxel step. A group with one rigid root bypasses the mini-BVH; flattening articulated leaves into scene acceleration remains benchmarkable. Derived encodings may change without changing canonical assets, pose graphs, or visual semantics.

### 6.3 Remaining evidence-gated choices

| Question                         | Candidate approaches                                                                                                      | Deciding experiment                                                                        |
| -------------------------------- | ------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| Local voxel search               | 64-tree, sparse octree with bricks, sparse brick grid                                                                     | Sparse foliage versus dense crystals: coherent + incoherent rays                           |
| 64-bit prefix bit operations     | Two-`u32` branchless/select baseline vs. conditional variant                                                              | Exact CPU equality for 64 indices, generated shader behavior and measured inner-loop cost  |
| Traversal state                  | Short private stack, stackless restart/bit trail, hybrid, staged traversal                                                | Registers, spills, stack visits, occupancy, and full GPU time                              |
| Scene broad phase                | Software BVH4, optional Vulkan hardware RT                                                                                | Overlap, dynamic refits, shadow rays and total frame cost                                  |
| Articulated spatial organization | Template-shared conservative object mini-BVH; per-instance refit; one scene leaf per group; bypass for single-root assets | Dynamic cluster count, envelope overlap, per-instance state refinement, memory, total rays |
| Resident pages                   | Adapter-legal contiguous single-slab hot path; typed page-table multi-slab fallback                                       | Vulkan/Metal requested limits; per-step dispatch impact; cross-slab edits                  |
| Node encoding                    | `lod_meta` leaf flag + logical word handles vs. alternative typed layout                                                  | Shader/CPU pack-unpack tests, page boundaries, slot exhaustion                             |
| Exact hit occurrence encoding    | Extra `R32Uint` local integer coordinate or compact occurrence/brick record                                               | Brick-boundary reference tests, shared DAG hits, wide world positions, bandwidth           |
| Dynamic bounds                   | Per-frame tight, demand-computed, hybrid cached                                                                           | Dense forest plus rare extreme gusts                                                       |
| Fine deformation                 | Rigid pose groups, affine shear, piecewise rigid, per-domain inverse field, forward update / flipbook fallback            | Thin curved silhouettes, motion vectors, performance, and no missed intersections          |
| Foliage filtering                | Axis coverage proxy, six-coefficient SGGX tensor, richer directional representation                                       | Supersampled angular coverage, opacity/transmittance, shadows, VRAM and bandwidth          |
| Temporal filtering               | Standard TAA, stronger reconstruction / denoising                                                                         | Thin animated features and topology changes                                                |
| Transparent crystals             | Material-boundary marching, dedicated refractive traversal, certified homogeneous-region skipping                         | Correct real interface/absorption behavior versus traversal cost                           |
| Animation LOD                    | None, distance tiers, projected-impact tiers                                                                              | Visual stability and measurable GPU savings                                                |

### 6.4 Required engineering diagnostics

- GPU timestamps per pass, rays/sec and candidate count per ray.
- WGSL occupancy-mask correctness versus CPU reference, generated GPU prefix code, register counts, spills, shared-memory allocation, occupancy, traversal-stack depth, and repeated node visits.
- False-positive bound visits and worst-case motion-envelope overlap.
- Cache/page miss counters, resident bytes, slab switches, cross-slab references, page fragmentation, and actual logical-geometry reuse.
- Pose-frame identity lookup count, active frame-table capacity, generation reuse, group-slot indirection, motion-vector and shading reconstruction for overlapping articulated groups.
- Exact terminal-cell or LOD-region reconstruction errors against reference, including boundary ties, negative ray directions, reused DAG nodes, and large world offsets; added identity-buffer bandwidth.
- Template-shared mini-BVH resident bytes, legal motion-envelope validity, candidate false-positive counts, and speed/memory comparison to per-instance refit and flattened groups.
- Mini-BVH versus flattened-group comparison under low/high articulation and large maximum-deformation envelopes.
- Comparison of visible detail against offline/supersampled reference frames, including angular opacity/transmittance of axis-coverage versus SGGX-style summaries.
- Refractive homogeneous-medium certificate validity, real material-interface locations, attenuation/path-length error, and crystals with internal cavities or edits.
- Rigid/affine/piecewise/nonlinear animation cost and exactness, including normal transformations and silhouette continuity.
- Temporal ghosting, silhouette errors, motion-vector validity and animation LOD transitions.
- Cross-platform screenshots and validation captures for the same asset, pose and camera.

**Acceptance standard:** A beautiful dense scene that remains stable in motion is a stronger result than a higher theoretical voxel count or a farther draw distance.

### Implementation documentation notes

- `wgpu::Features::EXPERIMENTAL_RAY_QUERY` remains an experimental, Vulkan-supported feature in the referenced current wgpu documentation, and must be capability-checked before use. See: https://docs.rs/wgpu/latest/wgpu/struct.Features.html and https://github.com/gfx-rs/wgpu/blob/trunk/docs/api-specs/ray_tracing.md
- Buffer packing and host/shader agreements must obey WGSL alignment/layout rules. See: https://www.w3.org/TR/WGSL/
- Adapter limit selection must use the **requested device limits**, not assume that clearing a constraint forces the adapter maximum: https://docs.rs/wgpu/latest/wgpu/struct.Limits.html and https://github.com/bevyengine/bevy/blob/main/crates/bevy_render/src/renderer/mod.rs
- Every numerical performance target in this document is a **measurement goal**, not a validated benchmark claim.
