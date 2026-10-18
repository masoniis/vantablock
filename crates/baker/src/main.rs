//! `vanta_baker`: bakes asset recipes into `.vanta` files and reports derived-geometry stats.
//!
//! ```text
//! vanta_baker [--out DIR] [--stats] RECIPE.ron...
//! ```
//!
//! With no recipes given, every `*.ron` recipe in `assets/shared/vanta` is baked.

use std::{
    path::{Path, PathBuf},
    process::ExitCode,
    time::Instant,
};
use voxel::{CanonicalAssembly, GeometryArenas, HierarchyKind, format};

fn main() -> ExitCode {
    let mut out_dir = PathBuf::from("target/vanta");
    let mut stats = false;
    let mut recipes = Vec::new();

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--out" => match args.next() {
                Some(dir) => out_dir = dir.into(),
                None => return usage("--out needs a directory"),
            },
            "--stats" => stats = true,
            "-h" | "--help" => return usage(""),
            _ => recipes.push(PathBuf::from(arg)),
        }
    }

    if recipes.is_empty() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/shared/vanta");
        match std::fs::read_dir(&dir) {
            Ok(entries) => recipes.extend(
                entries
                    .filter_map(|e| e.ok().map(|e| e.path()))
                    .filter(|p| p.extension().is_some_and(|e| e == "ron")),
            ),
            Err(e) => return usage(&format!("could not read {}: {e}", dir.display())),
        }
        recipes.sort();
    }

    if let Err(e) = std::fs::create_dir_all(&out_dir) {
        eprintln!("could not create {}: {e}", out_dir.display());
        return ExitCode::FAILURE;
    }

    for path in &recipes {
        if let Err(e) = bake_one(path, &out_dir, stats) {
            eprintln!("{}: {e}", path.display());
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}

fn bake_one(path: &Path, out_dir: &Path, stats: bool) -> Result<(), Box<dyn std::error::Error>> {
    let recipe = baker::parse_recipe(&std::fs::read_to_string(path)?)?;

    let start = Instant::now();
    let assembly = baker::bake_recipe(&recipe);
    let generated = start.elapsed();

    let bytes = format::write(&assembly)?;
    let out = out_dir.join(format!("{}.vanta", recipe.name));
    std::fs::write(&out, &bytes)?;

    let voxels: u64 = assembly.modules.iter().map(|m| m.voxel_count()).sum();
    println!(
        "{}: {} voxels, {} modules, {} pose groups, generated in {:.2?} -> {} ({:.1} KiB)",
        recipe.name,
        voxels,
        assembly.modules.len(),
        assembly.groups.len(),
        generated,
        out.display(),
        bytes.len() as f64 / 1024.0
    );

    if stats {
        for kind in HierarchyKind::ALL {
            print_hierarchy_stats(&assembly, kind)?;
        }
    }
    Ok(())
}

fn print_hierarchy_stats(
    assembly: &CanonicalAssembly,
    kind: HierarchyKind,
) -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();
    let mut arenas = GeometryArenas::default();
    let mut nodes = 0;
    let mut terminal = 0;
    for (i, module) in assembly.modules.iter().enumerate() {
        let (extent, single_page, s) = match kind {
            HierarchyKind::Tree64 => {
                let (root, s) = arenas.bake_tree64(module)?;
                (root.extent(), root.single_page, s)
            }
            HierarchyKind::Octree => {
                let (root, s) = arenas.bake_octree(module)?;
                (root.extent(), root.single_page, s)
            }
        };
        nodes += s.nodes;
        terminal += s.terminal_nodes;
        if kind == HierarchyKind::Tree64 {
            println!(
                "  module {i}: {}^3 grid, {} voxels, single page: {}",
                extent, s.voxels, single_page
            );
        }
    }
    let (t, a) = (arenas.topology.stats(), arenas.attributes.stats());
    println!(
        "  {kind:?}: {nodes} nodes ({terminal} {}), topology {:.1} KiB ({} blocks, {} shared), attributes {:.1} KiB ({} blocks, {} shared), {} LOD records, baked in {:.2?}",
        match kind {
            HierarchyKind::Tree64 => "terminal nodes",
            HierarchyKind::Octree => "bricks",
        },
        t.words as f64 * 4.0 / 1024.0,
        t.blocks,
        t.dedup_hits,
        a.words as f64 * 4.0 / 1024.0,
        a.blocks,
        a.dedup_hits,
        arenas.lod.records().len(),
        start.elapsed()
    );
    Ok(())
}

fn usage(error: &str) -> ExitCode {
    if !error.is_empty() {
        eprintln!("error: {error}\n");
    }
    eprintln!("usage: vanta_baker [--out DIR] [--stats] RECIPE.ron...");
    if error.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
