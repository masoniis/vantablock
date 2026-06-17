# Vantablock Client (`vantablock-client`)

The `vantablock-client` crate contains the graphical client for the Vantablock voxel engine. It is completely isolated from the authoritative server logic, communicating exclusively via the network protocol defined in the `shared` crate.

## Features Flags

- `dev`: Enables Bevy dev tools.
- `tracy`: Enables Tracy profiling for client-side performance tuning.
- `distribution`: Trims logging for release builds.
