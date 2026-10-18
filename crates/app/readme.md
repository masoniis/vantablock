# Vantablock App (`vantablock-app`)

The `vantablock-app` crate holds the executable entry points of the game. Instead of putting binary targets in the `client` or `server` crates, it composes the workspace based on the build target, and it owns the singleplayer orchestrator (a background server thread the client connects to over local UDP) and the client/server integration tests.

## Binaries

- **`vantablock`** (`src/bin/client.rs`): The standard game client executable. When run, this orchestrates the local player experience. It requires the `client` feature.
- **`vantablock-server`** (`src/bin/server.rs`): The headless dedicated server executable. It requires the `server` feature and completely omits all graphics libraries (like `winit` or `wgpu`) to ensure it can compile and run on lightweight Linux server environments.
