//! Asset recipes: the RON description a generator bakes into a canonical asset.

use crate::generators::{
    crystal::{self, CrystalParams},
    grotto::{self, GrottoParams},
    meadow::{self, MeadowParams},
    rotor::{self, RotorParams},
    tree::{self, TreeParams},
};
use serde::{Deserialize, Serialize};
use voxel::CanonicalAssembly;

/// One bakeable asset.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AssetRecipe {
    pub name: String,
    /// Meters per voxel.
    pub voxel_size: f32,
    pub seed: u64,
    pub kind: RecipeKind,
}

/// The generator and its parameters. Parameters not given in the recipe use their defaults.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum RecipeKind {
    Tree(TreeParams),
    Crystal(CrystalParams),
    Grotto(GrottoParams),
    Meadow(MeadowParams),
    Rotor(RotorParams),
}

pub fn parse_recipe(text: &str) -> Result<AssetRecipe, ron::error::SpannedError> {
    ron::from_str(text)
}

/// Deterministically bakes a recipe into its canonical asset.
pub fn bake_recipe(recipe: &AssetRecipe) -> CanonicalAssembly {
    let assembly = match &recipe.kind {
        RecipeKind::Tree(p) => tree::generate(recipe, p),
        RecipeKind::Crystal(p) => crystal::generate(recipe, p),
        RecipeKind::Grotto(p) => grotto::generate(recipe, p),
        RecipeKind::Meadow(p) => meadow::generate(recipe, p),
        RecipeKind::Rotor(p) => rotor::generate(recipe, p),
    };
    if let Err(e) = assembly.validate() {
        panic!(
            "generator for '{}' produced an invalid assembly: {e}",
            recipe.name
        );
    }
    assembly
}
