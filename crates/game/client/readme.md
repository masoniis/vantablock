# Vantablock Client (`vantablock-client`)

The `vantablock-client` crate contains the graphical client. It is isolated from the authoritative server logic, communicating exclusively via the network protocol defined in the `common` crate.

Its renderer (`src/render`) bakes the canonical asset library into GPU geometry, resolves replicated world objects into pose frames every frame, and ray traces them in compute shaders written in WESL (`assets/client/shader/voxel`).

## Testing

`tests/gpu_conformance.rs` validates every shader with naga and, when a GPU is available, holds the WGSL traversal to the CPU reference in `vantablock-voxel` (`just test-gpu`). `gpu_scene_throughput` (ignored by default) benchmarks the traversal kernel, and `gpu_primary_visibility_cost` the production primary visibility kernels with and without footprint LOD filtering.

## Features Flags

- `dev`: Enables Bevy dev tools.
- `tracy`: Enables Tracy profiling for client-side performance tuning.
- `distribution`: Trims logging for release builds.
- `brp`: Serves the Bevy Remote Protocol for agent and tool control.

Set `VANTA_GPU_TIMINGS=1` to enable Bevy's per-pass render diagnostics (on Metal these failed to allocate counter buffers and lost the device, so they are opt-in).
