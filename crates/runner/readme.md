# Vantablock Runner (`vantablock-runner`)

The `vantablock-runner` crate contains the actual executable entry points (`binaries`) for the Vantablock engine. Instead of putting binary targets in the `client` or `server` crates, this orchestration crate composes the workspace together based on the desired build target.

## Binaries

- **`vantablock`** (`src/bin/client.rs`): The standard game client executable. When run, this orchestrates the local player experience. It requires the `client` feature.
- **`vantablock-server`** (`src/bin/server.rs`): The headless dedicated server executable. It requires the `server` feature and completely omits all graphics libraries (like `winit` or `wgpu`) to ensure it can compile and run on lightweight Linux server environments.
