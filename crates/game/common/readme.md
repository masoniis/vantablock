# Vantablock Common (`vantablock-common`)

The `vantablock-common` crate is the game both peers run: the network protocol, the canonical asset library (baked on demand through `vantablock-baker`), replicated world object components, the world clock, shared player simulation, and the lifecycle loading DAG. `vantablock-server` and `vantablock-client` build on it; it never contains server-authoritative or rendering logic.
