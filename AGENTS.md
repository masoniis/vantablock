# VantaBlock Project Context

VantaBlock is a microvoxel engine built with Rust on Bevy, ray tracing dense, close-up worlds of canonical voxel assets with custom WGSL (WESL) compute shaders, and using a client/server architecture networked with lightyear.

## Project Overview

- **Core Technology:** Rust, Bevy (ECS, RenderApp), WGPU, WESL (WebGPU Shading Language), lightyear (networking), leafwing-input-manager (input).
- **Design:** Follows the engine design spec (v3.1). `docs/architecture.md` maps it onto the code and records what is built. Canonical geometry, assemblies, and animation state are the source of truth; hierarchies, bounds, LODs, and visibility records are derived and replaceable.
- **Architecture:** Multi-crate Cargo workspace grouped by layer. Dependencies only flow downward: `platform`, `engine/voxel` ← `engine/baker` ← `game/common` ← `game/server`/`game/client` ← `app`.
  - `crates/platform` (`vantablock-platform`): Host services with no game knowledge: logging, tracing, and persistent path management.
  - `crates/engine/` holds the Bevy-free libraries every peer and tool uses (never client-only or server-only):
    - `voxel` (`vantablock-voxel`): Geometry core: canonical modules and assemblies, `Node64`, masks, arenas, residency, the local hierarchies (64-tree, octree), BVH4 scene search, and the CPU reference traversal the shaders are held to.
    - `baker` (`vantablock-baker`): Procedural generators and RON recipes that bake canonical assets, the `vanta_baker` binary, and CPU traversal benchmarks.
  - `crates/game/` holds the Bevy plugins of the networked game:
    - `common` (`vantablock-common`): The game both peers run: the network protocol, the canonical asset library, replicated world objects, the world clock, shared player simulation, and the lifecycle/loading DAG.
    - `server` (`vantablock-server`): Authoritative simulation: scene population, the world clock, and connections.
    - `client` (`vantablock-client`): Input, UI, client-side prediction of the local player, and the voxel renderer.
  - `crates/app` (`vantablock-app`): The binaries (`vantablock`, `vantablock-server`), the singleplayer orchestrator, and client/server integration tests.

## Building and Running

The project uses `just` as a command runner. A Nix flake is provided to ensure a consistent development environment.

### Main Commands

- **Run Client (Debug):** `just run` or `just client`
- **Run Client (Release/Fast):** `just run-fast` or `just release`
- **Run Dedicated Server:** `just server`
- **Run Client with BRP (agent control):** `just brp` (or `just brp-build` to only build it)
- **Tests:** `just test` (runs `cargo nextest`)
- **GPU Conformance:** `just test-gpu` (naga validation, plus WGSL vs CPU reference on the real adapter)
- **Benchmarks:** `just bench` (CPU traversal), `just test-gpu gpu_scene_throughput --ignored` (GPU traversal per organization, hierarchy, and traversal-state variant), and `just test-gpu gpu_multi_slab_cost --ignored` (residency). Run GPU benchmarks one at a time (`--release` for stable numbers): parallel runs share the GPU and skew each other.
- **Bake Assets:** `just bake` (writes `.vanta` files and prints 64-tree stats)
- **Profiling:** `just trace` (requires Tracy profiler)
- **Packaging:** `just package` / `just server-package` (uses `cargo-packager` for distribution)

### Tooling & Environment

- **Worktrees:** Asset and dev-data paths are baked in at compile time from `CARGO_MANIFEST_DIR` (`crates/platform/src/paths.rs`). Never point a git worktree at the main checkout's `target/` (`CARGO_TARGET_DIR`): cargo can reuse the worktree's artifacts, and the main binary then loads assets from the worktree path. If that happens, run `cargo clean -p vantablock-platform`.
- **Dev Paths:** Dev asset paths are used when running under cargo or in debug builds. To launch a release binary directly (not via `cargo run`), set `CARGO=cargo` or it looks for assets next to the executable.
- **Searching:** Always exclude the `target/` directory when searching (e.g., `grep --exclude-dir=target`). It contains large build artifacts that can overwhelm search results and consume excessive context.
- **Environment Setup:** Use `nix develop` or `direnv allow` if using Nix.
- **Formatting:** `just fmt` (runs `nix fmt` which handles Rust and WGSL/WESL).
- **Linting:** `just clippy`.
- **Pre-push:** `just ready` (checks benches, clippy with `-D warnings`, and tests).

## Development Conventions

### ECS & Simulation

- **Separation of Concerns:** Keep simulation logic in `common` (when both sides run it) or `server` (when it is authoritative only). Rendering logic must be strictly separated into the client `render` module and extracted into Bevy's `RenderApp`. Geometry algorithms that don't need Bevy belong in `voxel`.
- **Fixed Timestep:** Simulation runs in `FixedUpdate` at `TICKS_PER_SECOND` (`crates/game/common/src/time`). Visuals are smoothed toward simulation state every frame in the client (`crates/game/client/src/network/graphics`); smoothing must stay frame-rate independent (exponential decay), since a plain `lerp(dt * rate)` diverges during slow frames.
- **World Clock:** The server advances `WorldClockResource` in `FixedUpdate` and broadcasts `SyncWorldClock`; clients advance it in `Update` and snap to syncs. Never advance it in client `FixedUpdate`, where prediction rollbacks re-run systems.

### Networking

- **Server Authoritative:** The server owns all game state. Singleplayer also goes through the network stack: the runner spins up a background server thread and the client connects over local UDP.
- **Prediction:** The server spawns players with `PredictionTarget` for the owner and `InterpolationTarget` for everyone else. On the client, `Predicted` identifies the local player and `Interpolated` identifies remote players. Never overwrite predicted components (`LogicalPosition`, `PlayerLook`) outside the fixed-step simulation, or the prediction history diverges and causes rollbacks.
- **Inputs:** Player intents go through leafwing `PlayerAction`s, which lightyear buffers and sends per tick. Write custom input values (such as look angles) in `InputSystems::WriteClientInputs` in `FixedPreUpdate`. Client-only actions (menus, debug views, clock requests) are `ClientAction`s on a global `ActionState<ClientAction>` resource.
- **Messages:** `MessageSender`/`MessageReceiver` components live on the connection entity (`Client` on the client, `ClientOf` on the server), not on player entities. Incoming messages are demultiplexed into local ECS messages (see the `network/receive` modules).
- **Protocol:** Channels, messages, and replicated components are registered in `crates/game/common/src/network/protocol`, after the lightyear client/server plugins.
- **World Objects:** The server spawns `WorldObject` + `ObjectPlacement` + `ObjectAnimation` entities from the scene layout and replicates them. Clients never spawn them, and poses are never streamed: clients evaluate `PoseDriver`s against the world clock. Assets are addressed by `VantaAssetId`, a stable hash of the asset name.
- **Session Lifetime:** Exiting `ClientLifecycleState::InGame` (to the menu or error screen) tears the session down: the client connection entity is despawned, the client scene is cleared, and the singleplayer background server is told to exit. Lightyear's prediction systems assume exactly one `Client` entity, so never leave a stale one behind.

### Geometry & Rendering

- **Assets:** Recipes in `assets/shared/vanta/*.ron` are baked on demand by both peers (`common::world::assets`) and cached as `.vanta` files keyed by recipe text and `baker::BAKER_VERSION`. Bump the version whenever a generator's output changes, or running games keep loading the stale cached bake. Asset root frames are anchored at the base center with +Y up.
- **Renderer:** `client::render` bakes every module as both local hierarchies (64-tree and octree) into one `GpuGeometryLibrary` during client launch, packs a `SceneSnapshot` into `GpuSceneData` every frame, and runs `PrimaryVisibility` (compute) then `DirectLighting` (fullscreen) in Bevy's `Core3d` graph for cameras marked `VoxelCamera`. `RenderSettings` picks the hierarchy and pose-group organization at runtime; `docs/architecture.md` records why the defaults are what they are.
- **Shader Variants:** `build.rs` compiles each shader once per WESL feature set: `multi_slab` (geometry spread over several storage slabs; the renderer picks it from the uploaded library) and, for conformance only, `stackless`. Anything that reads geometry must go through `topology_load`/`attribute_load` so both variants stay correct, and conformance must cover every variant.
- **CPU/GPU Parity:** The WGSL in `assets/client/shader/voxel/lib` mirrors `crates/engine/voxel` (masks, the `dda` rules, `tree64`, `octree`, `scene`, `math`). Change both together, keep `crates/game/client/src/render/gpu/layout.rs` in sync with `lib/types.wesl`, and run `just test-gpu`: shader changes are not done until the conformance tests pass on a real adapter.
- **Exact Identity:** Traversal reports integer voxel cells from traversal state. Never recover a cell or entry cell by flooring a reconstructed floating-point position; compare `t` against plane distances instead.
- **WGSL Pitfalls:** `attribute` and `handle` are reserved words; mixing bitwise and arithmetic or comparison operators needs parentheses; avoid dynamically indexing vector components in hot loops (it forces them out of registers). `wgslfmt` (run by `just fmt`) mangles statement-level WESL `@if` into an `if` statement, so put conditional compilation on whole declarations (`@if(feature) fn ...` with an `@if(!feature)` twin), and rebuild after formatting.
- **Optimization:** Prioritize data-oriented design and measure before replacing working algorithms. Judge GPU changes with `gpu_scene_throughput`, not the in-game FPS of a debug build.

### Testing

- **Integration Tests:** Client/server tests live in `crates/app/tests`. When driving apps manually with `app.update()`, call `app.finish()` and `app.cleanup()` first: `App::run` normally does this, and lightyear builds its replication systems in `Plugin::finish`.
- **Headless Client:** `HeadlessClientPlugins` runs the client without a window or renderer; the render plugin (and its scene/geometry resources) is absent.
- **Reference Tests:** Traversal changes must keep `crates/engine/voxel/tests` (boundary torture for every hierarchy and scene equivalence) passing; `voxel::reference::check_hit` defines what a correct hit is.

### Interactive Testing (BRP)

Unit and integration tests can't catch everything (UI flows, rendering, session lifecycle). Use the Bevy Remote Protocol to drive the real game and verify changes end to end.

- **Setup:** The client's `brp` cargo feature adds `bevy_brp_extras`, which serves BRP on `localhost:15702`. The `bevy_brp_mcp` server (in the nix devshell, or `cargo install bevy_brp_mcp --version 0.19.0`) is registered in `.mcp.json`. The `bevy_brp_*` versions must match the Bevy release (0.19.x for Bevy 0.18).
- **Workflow:** Run `just brp-build`, then launch with the `brp_launch` tool (target `vantablock`); it runs the already-built binary, so rebuild after code changes. Drive it with `brp_extras_*` input tools, inspect with `world_query`/`world_get_components`, check `brp_extras_screenshot` images, read logs with `brp_read_log`, and stop it with `brp_shutdown`. Make sure no earlier instance still holds the BRP port. Debug builds trace slowly enough that a screenshot can stall the client past the server's 10 s timeout; for longer sessions build with `cargo build --release -p vantablock-app --bin vantablock --features brp` and launch with profile `release` and `CARGO=cargo` in its environment. Movement input lags at low frame rates, so steer by reading the camera's `GlobalTransform` between short key presses.
- **Mouse Coordinates:** `move_mouse` positions are logical window pixels. Screenshots are physical pixels, so divide by the window scale factor (2.0 on Retina) to aim clicks.
- **Inspecting State:** Only types with `#[derive(Reflect)]` and `#[reflect(Component)]`/`#[reflect(Resource)]` are visible to BRP; add both to anything worth inspecting. `client::render::scene::ClientScene` holds scene stats, `client::render::settings::RenderSettings` switches debug views, acceleration modes, and render scale, and setting `client::ui::screens::debug_menu::DebugMenuState.visible` opens the debug menu (F3 is often captured by macOS), whose `Text` reports FPS and stats.
- **Debugging Renders:** Debug views separate failures quickly: `SceneNodes` and `GroupCandidates` show whether rays reach the scene BVH and pose frames, `TraversalSteps` shows DDA cost, `PoseFrames`/`VoxelCells` show identity, and `Motion` shows reconstructed screen-space motion.
- **Smoke Test:** After networking or lifecycle changes, check this loop: Singleplayer → hold W (position changes) → move mouse (`PlayerLook` follows) → Escape → Disconnect (lands on the main menu) → Singleplayer again, repeated a few times without a panic or a second `PredictionManager`.

### Tracing & Debugging

- **Tracy:** Integrated via features. Use `just trace` to profile the application.
- **Render Timings:** `VANTA_GPU_TIMINGS=1` enables Bevy's per-pass render diagnostics (opt-in: on Metal they lost the device).
- **Logging:** Use `tracing` macros (`info!`, `warn!`, `error!`). Use `just debug <target>` to enable specific trace-level logging for a module, or set `RUST_LOG` (e.g., `RUST_LOG=info,lightyear_inputs=trace`).

### Rust Style & Conventions

- **Grouped Imports:** Prefer grouped and collapsed `use` statements (e.g., `use crate::{module::{A, B}, other::C};`) to minimize vertical noise and keep the top of files concise.

## Workspace Structure

- `assets/`: Split into `client/` (fonts, WESL shaders), `server/` (scene layouts), and `shared/` (asset recipes both peers bake).
- `docs/`: `architecture.md` (the map from the engine design to the code) and `plan.md` (the engine design spec).
- `crates/`:
  - `engine/voxel/src/`: Canonical data model, `hierarchy/` (`tree64.rs`, `octree.rs`, shared DDA rules), `bvh.rs`, `scene.rs`, `reference.rs`.
  - `engine/baker/src/generators/`: Procedural asset generators.
  - `game/client/src/render/`: Geometry library, scene packing, GPU resources, and the render graph passes.
  - `game/client/src/network/`, `game/server/src/network/`: Connection handling, message demultiplexing, and sending.
  - `game/server/src/world/`: Scene population and the authoritative clock.
  - `game/common/src/network/protocol/`: Channels, messages, and replicated components.
  - `game/common/src/world/`: The asset library and replicated world object components.
  - `game/common/src/player/`: Shared player components and movement simulation.
  - `platform/src/logger.rs`: Centralized logging configuration.
- `target/`: Build artifacts (ignored by git).
