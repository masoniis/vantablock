# Vantablock Platform (`vantablock-platform`)

The `vantablock-platform` crate holds host services that know nothing about the game or the engine: it is what every binary sets up first.

## Features

- **Logging & Profiling**: Configures `tracing` subscribers and provides integration with `tracy-client` (via the `tracy` feature flag) for advanced memory and CPU allocation tracking.
- **System Information**: Interacts with the host OS to gather system specs or determine platform-specific file directories (using `sysinfo` and `directories`).
- **Paths**: Resolves asset, config, save, cache, and log directories across OS environments, with dev paths baked in from `CARGO_MANIFEST_DIR`.
