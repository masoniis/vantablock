//! Every shipped recipe must parse, bake into a valid assembly, and bake identically twice
//! (the asset cache and network asset IDs both rely on determinism).

use std::path::Path;

#[test]
fn shipped_recipes_bake_deterministically() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../assets/shared/vanta");
    let mut count = 0;
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "ron") {
            continue;
        }
        let recipe = baker::parse_recipe(&std::fs::read_to_string(&path).unwrap())
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let a = baker::bake_recipe(&recipe);
        let b = baker::bake_recipe(&recipe);
        assert!(a.validate().is_ok());
        assert!(
            a.modules.iter().all(|m| m.voxel_count() > 0),
            "{} has an empty module",
            recipe.name
        );
        assert_eq!(
            voxel::format::write(&a).unwrap(),
            voxel::format::write(&b).unwrap(),
            "{} is not deterministic",
            recipe.name
        );
        count += 1;
    }
    assert!(count >= 4);
}
