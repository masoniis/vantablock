# Vantablock Baker (`vantablock-baker`)

Procedural content generation. RON recipes in `assets/shared/vanta` name a generator (trees, crystal clusters, meadow tiles, kinetic rotors) and its parameters; baking one deterministically produces a canonical asset.

- `just bake` runs `vanta_baker`, which writes `.vanta` files and reports derived 64-tree statistics.
- `just bench` runs the CPU traversal benchmarks on the real assets.

Bump `BAKER_VERSION` whenever a generator's output changes, so cached bakes are invalidated.
