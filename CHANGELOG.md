# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.0] - 2026-10-28



### Bug Fixes 🐛

- ignore node completions after a loading phase is nuked
- keep position smoothing stable during slow frames
- stop mouse look while paused
- show the window only after the render world has warmed up
- leaving a session no longer panics
- set the input timeline config as a resource
- count the window reveal timeout from the first running frame
- meshing bugs and chunk module cleanup ([#46](https://github.com/masoniis/vantablock/pull/46))
- in-game keybinds; feat: new world terrain, showcase menu, sonder UI style ([#47](https://github.com/masoniis/vantablock/pull/47))
- stale chunk promotion leaving stuck meshing queue and double despawns ([#48](https://github.com/masoniis/vantablock/pull/48))
- stuck meshing queue; refactor chunk meshing and layering; server-authoritative clock ([#49](https://github.com/masoniis/vantablock/pull/49))
### Features 🚀

- rebuild the client UI in BSN with a themed look, a colored F3 overlay, and a controls list
### Build

- link bevy dynamically by default and optimize dev dependencies
## [0.3.0] - 2026-10-28



### Bug Fixes 🐛

- ignore node completions after a loading phase is nuked
- keep position smoothing stable during slow frames
- stop mouse look while paused
- show the window only after the render world has warmed up
- leaving a session no longer panics
- set the input timeline config as a resource
- count the window reveal timeout from the first running frame
- stale chunk promotion leaving stuck meshing queue and double despawns ([#48](https://github.com/masoniis/vantablock/pull/48))
- meshing bugs and chunk module cleanup ([#46](https://github.com/masoniis/vantablock/pull/46))
- in-game keybinds; feat: new world terrain, showcase menu, sonder UI style ([#47](https://github.com/masoniis/vantablock/pull/47))
- stuck meshing queue; refactor chunk meshing and layering; server-authoritative clock ([#49](https://github.com/masoniis/vantablock/pull/49))
### Features 🚀

- rebuild the client UI in BSN with a themed look, a colored F3 overlay, and a controls list
### Build

- link bevy dynamically by default and optimize dev dependencies
## [0.0.1] - 2026-04-09



### Features 🚀

- cross-platform clean asset management
- switch from release-please to release-plz
- integrate bevy_ui pipeline and set an initial simple startup screen ([#34](https://github.com/masoniis/vantablock/pull/34))