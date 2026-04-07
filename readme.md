# Vanta🅱️lock

[![GitHub Repository](https://img.shields.io/badge/GitHub-Repository-blue?style=flat&logo=github)](https://github.com/masoniis/vantablock)
![Rust Version](https://img.shields.io/badge/rustc-1.88.0%2B-orange.svg)

## Table of contents

- [Build and run the project](#how-to-run-the-project)
- [Usage guide](#usage-guide)
  - [Common problems](#common-problems)
  - [Keybinds](#keybinds)
- [Cool aspects of the project](#cool-aspects-of-the-project)
- [Acknowledgments](#acknowledgments)

## Build and run the project

<details>
  <summary>Nix users</summary>

Activate the [project's flake](./flake.nix) to set up the environment and all dependencies. For `direnv` users, the `.envrc` is set up to use the flake by default.

Then simply use any scripts from the `justfile`:

1. `just` to compile and run the debug binary (slow)
2. `just run-fast` to compile and run the release binary (fast)

</details>

<details>
  <summary>Other users</summary>

Compiling the project requires **Rust 1.88 or newer**.

- A `rust-toolchain.toml` is provided to ensure this for any users using `rustup` will automatically run with this version.
- Any other installation method will have to manually ensure the version 1.88 or newer.

Assuming rust is properly setup, `cargo` can be used like any standard Rust project:

- Run `cargo run --release` to compile and run in **release mode** (higher fps, optimized compilation)
- Run `cargo run` to compile and run in **debug mode** (lower FPS, debug tracing, simplified compilation)

</details>

## Usage guide

### Common problems

1. Mouse can be unlocked with `Escape`
2. Shadows have very low render distance of 32 voxels and also have other noticable issues.

### Keybinds

| Key(s)        | Action                                                                               |
| :------------ | :----------------------------------------------------------------------------------- |
| `Escape`      | Toggle "pause" (locks/unlocks cursor, no _real_ pause currently)                     |
| `W`           | Move forward                                                                         |
| `S`           | Move backward                                                                        |
| `A`           | Move left                                                                            |
| `D`           | Move right                                                                           |
| `Left Shift`  | Move faster                                                                          |
| `Mouse Left`  | Break voxel                                                                          |
| `Mouse right` | Place voxel                                                                          |
| `T`           | Switch terrain generator (only applies to new chunks that generate e.g. from moving) |
| `F1` or `1`   | Toggle diagnostics UI (FPS, vert count, coordinates)                                 |
| `F2` or `2`   | Toggle opaque wireframe mode                                                         |
| `F3` or `3`   | Toggle chunk borders                                                                 |

## Cool aspects of the project

<details>
  <summary>Computer graphics technical details</summary>

1. "Vertex pulling." Each group of 6 vertices that make up a face share a single 32 bit float in a GPU buffer.
2. Global illumination via the "Sun" with directional lighting, and a shadow pass that adds (somewhat scuffed) shadows
3. Approximate ambient occlusion based on nearby voxels to a vertex.
4. Full transparency support via a separate render pass.
5. Custom UI implementation (with `taffy` for computing flexbox layouts and `glyphon` for text heavylifting)
6. Custom fog and sky shaders that define the sky and horizon blending with sun/moon.
7. Convenient texture and voxel definition loading enabling swapping voxel textures easily in the `assets/shared/blocks` folder.
8. Water vertices "wave" up and down if you look at them closer

</details>

<details>
  <summary>General engine technical details</summary>

1. Chunk loading uses multi-threaded compute pooling
2. ECS architecture for data-oriented design of the entire system
3. Rendering and simulation run in parallel using a binary semaphore

</details>
