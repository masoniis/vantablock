# VantaBlock Project Context

VantaBlock is a high-performance, block-based game engine built with Rust, leveraging the Bevy game engine, custom WebGPU-based rendering techniques, and a client/server architecture networked with lightyear.

## Project Overview

- **Core Technology:** Rust, Bevy (ECS), WGPU, WESL (WebGPU Shading Language), lightyear (networking), leafwing-input-manager (input).
- **Architecture:** Multi-crate Cargo workspace:
  - `crates/game/client`: Frontend, UI, custom rendering pipeline, input, and client-side prediction of the local player.
  - `crates/game/server`: Authoritative simulation, including terrain generation, chunk loading, and block edits.
  - `crates/game/common`: Code both sides need: the network protocol, shared player simulation, chunk data, block/biome registries, and the lifecycle/loading DAG.
  - `crates/app`: The binaries (`vantablock`, `vantablock-server`), the singleplayer orchestrator, and client/server integration tests.
  - `crates/platform`: Common utilities including logging, tracing, and global path management.
- **Key Features:**
  - **Vertex Pulling:** Optimized face rendering where 6 vertices share a single 32-bit float in a GPU buffer.
  - **Custom Render Graph:** Specialized passes for opaque, transparent, skybox, and shadow rendering.
  - **Terrain Generation:** Multi-threaded, server-side chunk generation using noise-based biomes.
  - **Global Illumination:** Sun-based directional lighting with shadow mapping and ambient occlusion.

## Building and Running

The project uses `just` as a command runner. A Nix flake is provided to ensure a consistent development environment.

### Main Commands

- **Run Client (Debug):** `just run` or `just client`
- **Run Client (Release/Fast):** `just run-fast` or `just release`
- **Run Dedicated Server:** `just server`
- **Run Client with BRP (agent control):** `just brp` (or `just brp-build` to only build it)
- **Tests:** `just test` (runs `cargo nextest`)
- **Benchmarks:** `just bench-client` / `just bench-server` (opens HTML report in browser)
- **Profiling:** `just trace` (requires Tracy profiler)
- **Packaging:** `just package` / `just server-package` (uses `cargo-packager` for distribution)

### Tooling & Environment

- **Worktrees:** Asset and dev-data paths are baked in at compile time from `CARGO_MANIFEST_DIR` (`crates/platform/src/paths.rs`). Never point a git worktree at the main checkout's `target/` (`CARGO_TARGET_DIR`): cargo can reuse the worktree's artifacts, and the main binary then loads assets from the worktree path. If that happens, run `cargo clean -p vantablock-platform`.
- **Fast Builds:** The Bevy crates (`common`, `server`, `client`, `app`) default to `bevy/dynamic_linking`, and dev builds optimize dependencies. Plain `cargo`/`just` commands get all of this. Distribution builds (`just release`, `just package`, the release workflow) pass `--no-default-features --features client,distribution` to link statically; keep that on any new distribution path. The `release` profile has no LTO because it can't link against the Bevy dylib.
- **Searching:** Always exclude the `target/` directory when searching (e.g., `grep --exclude-dir=target`). It contains large build artifacts that can overwhelm search results and consume excessive context.
- **Environment Setup:** Use `nix develop` or `direnv allow` if using Nix.
- **Formatting:** `just fmt` (runs `nix fmt` which handles Rust and WGSL/WESL).
- **Linting:** `just clippy`.
- **Pre-push:** `just ready` (checks benches, clippy with `-D warnings`, and tests).

## Development Conventions

### ECS & Simulation

- **Separation of Concerns:** Keep simulation logic in `common` (when both sides run it) or `server` (when it is authoritative only). Rendering logic must be strictly separated into the client `render` modules and extracted into Bevy's `RenderApp`.
- **Fixed Timestep:** Simulation runs in `FixedUpdate` at `TICKS_PER_SECOND` (`crates/game/common/src/time`). Visuals are smoothed toward simulation state every frame in the client (`crates/game/client/src/network/graphics`).
- **Entity Synchronization:** Meshes and world data are synchronized to the render world via `SyncToRenderWorld` and custom extraction systems (see `crates/game/client/src/render/mod.rs`).

### Networking

- **Server Authoritative:** The server owns all game state. Singleplayer also goes through the network stack: the runner spins up a background server thread and the client connects over local UDP.
- **Prediction:** The server spawns players with `PredictionTarget` for the owner and `InterpolationTarget` for everyone else. On the client, `Predicted` identifies the local player and `Interpolated` identifies remote players. Never overwrite predicted components (`LogicalPosition`, `PlayerLook`) outside the fixed-step simulation, or the prediction history diverges and causes rollbacks.
- **Inputs:** Player intents go through leafwing `PlayerAction`s, which lightyear buffers and sends per tick. Write custom input values (such as look angles) in `InputSystems::WriteClientInputs` in `FixedPreUpdate`.
- **Messages:** `MessageSender`/`MessageReceiver` components live on the connection entity (`Client` on the client, `ClientOf` on the server), not on player entities. Incoming messages are demultiplexed into local ECS messages (see the `network/receive` modules). The client mirrors this for outgoing traffic: gameplay systems write `Outbound*Message`s and `network/send` forwards them, so `MessageSender` is only touched there (the exception is `RequestChunks`, see the `network/send` module docs). Register every local message type with `add_message`; `init_resource::<Messages<_>>` never clears its queue.
- **Protocol:** Channels, messages, and replicated components are registered in `crates/game/common/src/network/protocol`, after the lightyear client/server plugins.
- **Chunk Streaming:** The client is the source of truth for which chunks it holds. It sends `RequestChunks` whenever it starts awaiting chunks (including ones it unloaded and needs again). The server queues requests per client in `ClientChunkTracker` and sends generated ones nearest-first. Never track "already sent" chunks on the server: the client unloads chunks on its own, and stale server state leaves permanent holes.
- **Session Lifetime:** Exiting `ClientLifecycleState::InGame` (to the menu or error screen) tears the session down: the client connection entity is despawned, client chunks are cleared, and the singleplayer background server is told to exit. Lightyear's prediction systems assume exactly one `Client` entity, so never leave a stale one behind. `PredictionManager` is a global resource in lightyear 0.30 (inserted per session, removed in `end_session`), never a component of the client entity: Bevy 0.19 stores resources on entities, so spawning it with the client made despawning the client panic.

### Graphics & Shaders

- **UI:** Screens are `bsn!` scenes composed from `ui::theme` (palette, fonts, text) and `ui::widgets` (buttons, text fields, keycaps), with behavior attached through `on(...)` observers. The pause menu's controls list is read from `input::input_maps::{PLAYER_KEYS, CLIENT_KEYS}`, which also build the input maps.
- **Shaders:** Written in WESL (`assets/client/shader/`). The `VantablockShaderPlugin` handles shader registration.
- **Assets:** Configured via RON files (e.g., `assets/server/config.ron`, `assets/shared/block/*.ron`, `assets/shared/biome/`).
- **Optimization:** Prioritize data-oriented design (DOD) to keep the "hot path" (rendering and chunk generation) efficient.

### Testing

- **Integration Tests:** Client/server tests live in `crates/app/tests`. When driving apps manually with `app.update()`, call `app.finish()` and `app.cleanup()` first: `App::run` normally does this, and lightyear builds its replication systems in `Plugin::finish`.
- **Headless Client:** `HeadlessClientPlugins` runs the client without a window or renderer, so render-owned resources like `ClientChunkManager` must be initialized by the test.

### Interactive Testing (BRP)

Unit and integration tests can't catch everything (UI flows, rendering, session lifecycle). Use the Bevy Remote Protocol to drive the real game and verify changes end to end.

- **Setup:** The client's `brp` cargo feature adds `bevy_brp_extras`, which serves BRP on `localhost:15702`. The `bevy_brp_mcp` server (in the nix devshell, or `cargo install bevy_brp_mcp --version 0.19.0`) is registered in `.mcp.json`. The `bevy_brp_*` versions must match the Bevy release (0.22.x for Bevy 0.19).
- **Workflow:** Run `just brp-build`, then launch with the `brp_launch` tool (target `vantablock`); it runs the already-built binary, so rebuild after code changes. Drive it with `brp_extras_*` input tools, inspect with `world_query`/`world_get_components`, check `brp_extras_screenshot` images, read logs with `brp_read_log`, and stop it with `brp_shutdown`.
- **Without MCP:** BRP is plain JSON-RPC over HTTP, e.g. `curl -s -X POST localhost:15702 -d '{"jsonrpc":"2.0","id":1,"method":"world.query","params":{"data":{},"filter":{"with":["client::player::components::LocalPlayer"]}}}'`.
- **Mouse Coordinates:** `move_mouse` positions are logical window pixels. Screenshots are physical pixels, so divide by the window scale factor (2.0 on Retina) to aim clicks.
- **Inspecting State:** Only components with `#[derive(Reflect)]` and `#[reflect(Component)]` are visible to BRP; add both to anything worth inspecting. App states are not reflected, but the F3 debug menu's text (position, chunk counts, chunks awaiting server data, meshing queue) can be read with a `world_query` on `bevy_ui::widget::text::TextSpan`, and setting `client::ui::screens::debug_menu::DebugMenuState.visible` opens it (F3 is often captured by macOS).
- **Expectations:** In debug builds, terrain takes several seconds to mesh after joining, so an early screenshot showing only sky is normal. Check the meshing queue before concluding rendering is broken.
- **Smoke Test:** After networking or lifecycle changes, check this loop: Singleplayer → hold W (position changes) → move mouse (`PlayerLook` follows) → Escape → Disconnect (lands on the main menu) → Singleplayer again, repeated a few times without a panic or a second `PredictionManager`.

### Tracing & Debugging

- **Tracy:** Integrated via features. Use `just trace` to profile the application.
- **Logging:** Use `tracing` macros (`info!`, `warn!`, `error!`). Use `just debug <target>` to enable specific trace-level logging for a module, or set `RUST_LOG` (e.g., `RUST_LOG=info,lightyear_inputs=trace`).

### Rust Style & Conventions

- **Grouped Imports:** Prefer grouped and collapsed `use` statements (e.g., `use crate::{module::{A, B}, other::C};`) to minimize vertical noise and keep the top of files concise.

## Workspace Structure

- `assets/`: Split into `client/` (fonts, shaders, textures), `server/` (config), and `shared/` (blocks, biomes).
- `crates/`:
  - `game/client/src/render/`: Custom render passes and GPU resource management.
  - `game/client/src/network/`, `game/server/src/network/`: Connection handling, message demultiplexing, and sending.
  - `game/server/src/world/`: Terrain generation and chunk loading.
  - `game/common/src/network/protocol/`: Channels, messages, and replicated components.
  - `game/common/src/player/`: Shared player components and movement simulation.
  - `platform/src/logger.rs`: Centralized logging configuration.
- `target/`: Build artifacts (ignored by git).
