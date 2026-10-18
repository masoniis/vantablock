# Vantablock Voxel (`vantablock-voxel`)

The bevy-free geometry core. It holds the engine's data contracts (canonical modules and assemblies, `Node64`, masks, arena handles, residency), the 64-tree bake, BVH4 scene search, and the CPU reference traversal that the GPU shaders are tested against. See `docs/architecture.md`.

The crate is optimized in dev builds (`opt-level = 3`) because asset baking and the reference tests are hot.
