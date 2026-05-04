# Vantablock Utils (`vantablock-utils`)

The `vantablock-utils` crate provides generic helper functions and system integrations that are not tied to the specific game domain (like blocks or biomes).

## Features

- **Logging & Profiling**: Configures `tracing` subscribers and provides integration with `tracy-client` (via the `tracy` feature flag) for advanced memory and CPU allocation tracking.
- **System Information**: Interacts with the host OS to gather system specs or determine platform-specific file directories (using `sysinfo` and `directories`).
- **Paths**: Helper constants and functions for resolving asset and configuration paths across different OS environments.
