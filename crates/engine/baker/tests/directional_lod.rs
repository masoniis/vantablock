//! The directional LOD comparison (spec section 2.4 and the additional Phase 2 test): how well
//! each compact summary predicts the measured angular coverage of real baked nodes.
//!
//! Ignored by default (it measures thousands of nodes); run it with
//! `cargo test --release -p vantablock-baker --test directional_lod -- --ignored --nocapture`.

use glam::{UVec3, Vec3};
use std::{collections::BTreeSet, time::Instant};
use voxel::{
    angular::{
        BAKE_MEASUREMENT, BakeMeasurement, BakeSetup, EvenPoly4, NodeVoxels, Sggx, axis_opacity,
        hemisphere_directions,
    },
    lod::stored_tensor,
};

/// Directions the bake-time fits see, and the disjoint set every candidate is scored on.
const FIT_DIRECTIONS: usize = 48;
const EVAL_DIRECTIONS: usize = 96;
/// Rays per direction (`SAMPLES x SAMPLES`): about 1% sampling error at worst.
const SAMPLES: u32 = 48;
/// The most nodes measured per asset and node size, spread evenly over the asset.
const MAX_NODES: usize = 160;

fn recipe(name: &str) -> voxel::CanonicalAssembly {
    let path = format!(
        "{}/../../../assets/shared/vanta/{name}.ron",
        env!("CARGO_MANIFEST_DIR")
    );
    baker::bake_recipe(&baker::parse_recipe(&std::fs::read_to_string(path).unwrap()).unwrap())
}

/// Up to `MAX_NODES` occupied nodes `size` voxels on a side, spread over every module.
fn sample_nodes(assembly: &voxel::CanonicalAssembly, size: u32) -> Vec<NodeVoxels> {
    let mut corners = Vec::new();
    for (m, module) in assembly.modules.iter().enumerate() {
        let set: BTreeSet<(u32, u32, u32)> = module
            .bricks()
            .keys()
            .map(|&b| {
                let c = b * 4 / size * size;
                (c.z, c.y, c.x)
            })
            .collect();
        corners.extend(set.into_iter().map(|(z, y, x)| (m, UVec3::new(x, y, z))));
    }
    let stride = corners.len().div_ceil(MAX_NODES).max(1);
    corners
        .into_iter()
        .step_by(stride)
        .filter_map(|(m, c)| NodeVoxels::from_module(&assembly.modules[m], c, size))
        .collect()
}

/// Every candidate's predictions next to the measurement for one node.
struct NodeResult {
    truth: Vec<f32>,
    candidates: [Vec<f32>; CANDIDATES],
    /// Seconds the bake's own measurement and fit took.
    bake_seconds: f64,
}

const CANDIDATES: usize = 6;
const NAMES: [&str; CANDIDATES] = [
    "axis proxy (L1)",
    "diagonal SGGX (L2)",
    "fitted SGGX (f16)",
    "axis-fixed SGGX",
    "baked SGGX",
    "quartic (15 coef)",
];

/// The bake's measurement, overridable for exploration with `VANTA_BAKE_DIRECTIONS` and
/// `VANTA_BAKE_SAMPLES`.
fn bake_measurement() -> BakeMeasurement {
    let var = |name: &str| std::env::var(name).ok().and_then(|v| v.parse().ok());
    BakeMeasurement {
        directions: var("VANTA_BAKE_DIRECTIONS").unwrap_or(BAKE_MEASUREMENT.directions),
        samples: var("VANTA_BAKE_SAMPLES")
            .map(|v: usize| v as u32)
            .unwrap_or(BAKE_MEASUREMENT.samples),
        brick_grid: var("VANTA_BAKE_BRICK_GRID")
            .map(|v: usize| v as u32)
            .unwrap_or(BAKE_MEASUREMENT.brick_grid),
    }
}

fn measure(
    node: &NodeVoxels,
    fit_dirs: &[Vec3],
    eval_dirs: &[Vec3],
    bake: &BakeSetup,
) -> NodeResult {
    let fit: Vec<f32> = fit_dirs
        .iter()
        .map(|&w| node.coverage(w, SAMPLES))
        .collect();
    let truth: Vec<f32> = eval_dirs
        .iter()
        .map(|&w| node.coverage(w, SAMPLES))
        .collect();
    let axis = node.axis_coverage();
    let diagonal = Sggx::from_axis_coverage(axis);
    let sggx = Sggx::fit(fit_dirs, &fit).quantized();
    let axis_fixed = stored_tensor(Sggx::fit_with_axes(axis, fit_dirs, &fit));
    let start = Instant::now();
    let baked = stored_tensor(bake.tensor(node));
    let bake_seconds = start.elapsed().as_secs_f64();
    let quartic = EvenPoly4::fit(fit_dirs, &fit);
    let eval = |f: &dyn Fn(Vec3) -> f32| eval_dirs.iter().map(|&w| f(w)).collect::<Vec<_>>();
    NodeResult {
        truth,
        candidates: [
            eval(&|w| axis_opacity(axis, w)),
            eval(&|w| diagonal.opacity(w)),
            eval(&|w| sggx.opacity(w)),
            eval(&|w| axis_fixed.opacity(w)),
            eval(&|w| baked.opacity(w)),
            eval(&|w| quartic.opacity(w)),
        ],
        bake_seconds,
    }
}

fn measure_all(nodes: &[NodeVoxels]) -> Vec<NodeResult> {
    let fit_dirs = hemisphere_directions(FIT_DIRECTIONS, 0.0);
    let eval_dirs = hemisphere_directions(EVAL_DIRECTIONS, 1.234);
    let bake = BakeSetup::new(bake_measurement());
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let chunk = nodes.len().div_ceil(threads).max(1);
    std::thread::scope(|scope| {
        let handles: Vec<_> = nodes
            .chunks(chunk)
            .map(|part| {
                let (fit_dirs, eval_dirs, bake) = (&fit_dirs, &eval_dirs, &bake);
                scope.spawn(move || {
                    part.iter()
                        .map(|n| measure(n, fit_dirs, eval_dirs, bake))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap())
            .collect()
    })
}

fn report(label: &str, nodes: &[NodeVoxels]) {
    if nodes.is_empty() {
        return;
    }
    let start = Instant::now();
    let results = measure_all(nodes);
    let elapsed = start.elapsed();
    // nodes whose coverage varies with direction (the ones a directional summary is for)
    let anisotropic = results
        .iter()
        .filter(|r| {
            let (lo, hi) = r
                .truth
                .iter()
                .fold((1f32, 0f32), |(lo, hi), &c| (lo.min(c), hi.max(c)));
            hi - lo > 0.2
        })
        .count();
    let mean_truth = results.iter().flat_map(|r| &r.truth).sum::<f32>()
        / (results.len() * EVAL_DIRECTIONS) as f32;
    println!(
        "  {label}: {} nodes ({} anisotropic), mean coverage {:.3}, {:.1} ms per node, bake fit {:.1} us per node",
        results.len(),
        anisotropic,
        mean_truth,
        elapsed.as_secs_f64() * 1000.0 / results.len() as f64,
        results.iter().map(|r| r.bake_seconds).sum::<f64>() * 1e6 / results.len() as f64
    );
    for (c, name) in NAMES.iter().enumerate() {
        let mut errors: Vec<f32> = Vec::new();
        let mut signed = 0.0;
        let mut flips = 0usize;
        for r in &results {
            for (&t, &p) in r.truth.iter().zip(&r.candidates[c]) {
                errors.push((p - t).abs());
                signed += p - t;
                // the deterministic filter's hit/pass decision at threshold 0.5
                flips += ((p > 0.5) != (t > 0.5)) as usize;
            }
        }
        let n = errors.len() as f32;
        errors.sort_unstable_by(f32::total_cmp);
        let mean = errors.iter().sum::<f32>() / n;
        let rms = (errors.iter().map(|e| e * e).sum::<f32>() / n).sqrt();
        let p95 = errors[((n * 0.95) as usize).min(errors.len() - 1)];
        println!(
            "    {name:<20} mean |err| {mean:.3}, rms {rms:.3}, p95 {p95:.3}, bias {:+.3}, 0.5 flips {:.1}%",
            signed / n,
            flips as f32 / n * 100.0
        );
    }
}

#[test]
#[ignore = "measurement: run with --release --ignored --nocapture"]
fn directional_lod_comparison() {
    let bake = bake_measurement();
    println!(
        "baked SGGX: {} directions, {}x{} rays each ({}x{} brick lines)",
        bake.directions, bake.samples, bake.samples, bake.brick_grid, bake.brick_grid
    );
    println!("synthetic:");
    let leaf = |size: u32| {
        NodeVoxels::from_fn(size, move |p| {
            p.x == p.y && p.z >= size / 8 && p.z < size * 7 / 8
        })
    };
    let tilted = |size: u32| {
        NodeVoxels::from_fn(size, move |p| {
            let s = (p.x + p.y + p.z) as i32 - 3 * size as i32 / 2;
            (0..2).contains(&s)
        })
    };
    let synthetic: Vec<NodeVoxels> = [4, 16, 64]
        .into_iter()
        .flat_map(|s| [leaf(s), tilted(s)])
        .flatten()
        .collect();
    report("diagonal and tilted leaves", &synthetic);

    for name in ["oak", "crystal_grotto", "meadow_tile"] {
        let assembly = recipe(name);
        println!("{name}:");
        for size in [4, 16, 64] {
            report(&format!("{size}^3 nodes"), &sample_nodes(&assembly, size));
        }
    }
}
