//! CPU reference traversal throughput on the real baked assets.
//!
//! These numbers measure the reference implementation, not the GPU renderer; they exist to catch
//! regressions in the shared algorithm and to compare hierarchy and scene-organization choices.

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use glam::{Affine3A, Quat, Vec3};
use std::hint::black_box;
use voxel::{
    GeometryArenas, HierarchyKind, PoseDriver, Ray, Tree64Root,
    hierarchy::tree64::trace,
    scene::{AssetTemplate, InstanceDesc, SceneAcceleration, SceneSnapshot},
};

fn recipe(name: &str) -> voxel::CanonicalAssembly {
    let path = format!(
        "{}/../../assets/shared/vanta/{name}.ron",
        env!("CARGO_MANIFEST_DIR")
    );
    baker::bake_recipe(&baker::parse_recipe(&std::fs::read_to_string(path).unwrap()).unwrap())
}

/// A pinhole camera's rays over a `size x size` image, looking at `target` from `eye`.
fn camera_rays(eye: Vec3, target: Vec3, size: u32) -> Vec<Ray> {
    let forward = (target - eye).normalize();
    let right = forward.cross(Vec3::Y).normalize();
    let up = right.cross(forward);
    (0..size * size)
        .map(|i| {
            let u = (i % size) as f32 / size as f32 - 0.5;
            let v = (i / size) as f32 / size as f32 - 0.5;
            Ray::new(eye, (forward + right * u * 0.9 + up * v * 0.9).normalize())
        })
        .collect()
}

fn single_root(c: &mut Criterion) {
    let mut group = c.benchmark_group("local_hierarchy");
    for name in ["oak", "crystal_cluster", "meadow_tile"] {
        let assembly = recipe(name);
        for kind in HierarchyKind::ALL {
            let mut arenas = GeometryArenas::default();
            let root = arenas.bake(&assembly.modules[0], kind).unwrap();
            let bounds = root.bounds();
            let center = bounds.center();
            let rays = camera_rays(
                center + Vec3::new(0.0, 40.0, -bounds.extent().max_element()),
                center,
                128,
            );
            group.throughput(Throughput::Elements(rays.len() as u64));
            group.bench_function(format!("{name}_{kind:?}_primary_128x128"), |b| {
                b.iter(|| {
                    rays.iter()
                        .filter(|r| root.trace(&arenas, r, 0.0, f32::MAX).is_some())
                        .count()
                })
            });
        }
    }
    group.finish();
}

fn scene_modes(c: &mut Criterion) {
    let assemblies = [recipe("oak"), recipe("kinetic_rotor")];
    let mut arenas = GeometryArenas::default();
    let roots: Vec<Vec<Tree64Root>> = assemblies
        .iter()
        .map(|a| {
            a.modules
                .iter()
                .map(|m| arenas.bake_tree64(m).unwrap().0)
                .collect()
        })
        .collect();
    let templates: Vec<AssetTemplate> = assemblies.iter().map(AssetTemplate::new).collect();

    // a grove of overlapping, rotated, animated instances
    let instances: Vec<InstanceDesc> = (0..200)
        .map(|i| {
            let asset = (i % 2) as u32;
            let a = &assemblies[asset as usize];
            let angle = i as f32 * 2.399;
            let position = Vec3::new(angle.cos(), 0.0, angle.sin()) * (i as f32).sqrt() * 3.0;
            InstanceDesc {
                key: i as u64,
                asset,
                root_to_world: Affine3A::from_scale_rotation_translation(
                    Vec3::splat(a.voxel_size),
                    Quat::from_rotation_y(angle),
                    position,
                ),
                pose: PoseDriver {
                    phase: angle,
                    gust: 1.0,
                }
                .evaluate(a, 1.3),
            }
        })
        .collect();
    let rays = camera_rays(Vec3::new(0.0, 6.0, -45.0), Vec3::new(0.0, 4.0, 0.0), 96);

    let mut group = c.benchmark_group("scene_snapshot_build");
    for mode in [
        SceneAcceleration::TemplateMiniBvh,
        SceneAcceleration::FlattenedGroups,
    ] {
        group.bench_function(format!("{mode:?}_200_instances"), |b| {
            b.iter(|| SceneSnapshot::build(&templates, black_box(&instances), mode, None))
        });
    }
    group.finish();

    let mut group = c.benchmark_group("scene_trace");
    group.throughput(Throughput::Elements(rays.len() as u64));
    for mode in [
        SceneAcceleration::TemplateMiniBvh,
        SceneAcceleration::FlattenedGroups,
    ] {
        let snapshot = SceneSnapshot::build(&templates, &instances, mode, None);
        group.bench_function(format!("{mode:?}_96x96"), |b| {
            b.iter(|| {
                rays.iter()
                    .filter(|ray| {
                        snapshot
                            .intersect(&templates, ray, 0.0, f32::MAX, |frame, local, t0, t1| {
                                let module = assemblies[frame.asset as usize].groups
                                    [frame.group as usize]
                                    .module;
                                trace(
                                    &arenas,
                                    &roots[frame.asset as usize][module as usize],
                                    local,
                                    t0,
                                    t1,
                                )
                            })
                            .0
                            .is_some()
                    })
                    .count()
            })
        });
    }
    group.finish();
}

criterion_group!(benches, single_root, scene_modes);
criterion_main!(benches);
