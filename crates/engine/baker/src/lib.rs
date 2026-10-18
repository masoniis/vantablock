//! # The Vantablock Content Baker
//!
//! Turns procedural recipes into canonical assets. A recipe (RON) names a generator and its
//! parameters; baking it deterministically produces a [`voxel::CanonicalAssembly`], which is the
//! asset's source of truth and what `.vanta` files store.
//!
//! The `vanta_baker` binary bakes recipes to `.vanta` files and reports derived-hierarchy
//! statistics. The game itself bakes recipes on demand and caches the results.

pub mod generators;
pub mod noise;
pub mod recipe;
pub mod voxelize;

pub use recipe::{AssetRecipe, RecipeKind, bake_recipe, parse_recipe};

/// Bump whenever any generator's output changes, so cached `.vanta` bakes are invalidated.
pub const BAKER_VERSION: u32 = 2;
