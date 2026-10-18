//! Scene-level contracts: flattened pose-group leaves, template-shared mini-BVHs, and
//! per-instance refit mini-BVHs must find exactly the same nearest hit as an exhaustive search over
//! every resolved frame, for every legal pose and both local hierarchies, and rotated groups must
//! stay hittable.

mod common;

use common::{Rng, foliage_module};
use glam::{Affine3A, Quat, UVec3, Vec3};
use voxel::{
    CanonicalAssembly, GeometryArenas, HierarchyKind, Material, Motion, PoseDriver,
    PoseGroupTemplate, PoseState, Ray, VoxelModule, VoxelRoot,
    scene::{AssetTemplate, InstanceDesc, SceneAcceleration, SceneHit, SceneSnapshot},
};

/// The spec's minimal articulated asset: a static base plus two groups sharing one blade
/// module under two differently rotating transforms.
fn rotor_asset() -> CanonicalAssembly {
    let mut base = VoxelModule::new(3);
    for x in 24..40 {
        for z in 24..40 {
            for y in 0..6 {
                base.set(UVec3::new(x, y, z), 1);
            }
        }
    }
    let mut blade = VoxelModule::new(3);
    for x in 0..60 {
        for z in 30..34 {
            blade.set(UVec3::new(x, 30 + (x % 3), z), 2);
        }
    }
    let pivot = Vec3::new(32.0, 31.0, 32.0);
    CanonicalAssembly {
        name: "rotor".into(),
        voxel_size: 1.0 / 32.0,
        palette: vec![
            Material::new([0.0; 3]),
            Material::new([0.5; 3]),
            Material::new([0.9, 0.3, 0.2]),
        ],
        modules: vec![base, blade],
        groups: vec![
            PoseGroupTemplate {
                name: "base".into(),
                parent: None,
                module: 0,
                rest: Affine3A::IDENTITY,
                motion: Motion::Static,
            },
            PoseGroupTemplate {
                name: "rotor_a".into(),
                parent: Some(0),
                module: 1,
                rest: Affine3A::from_translation(Vec3::new(0.0, 10.0, 0.0)),
                motion: Motion::Spin {
                    pivot,
                    axis: Vec3::Y,
                    rate: 1.3,
                },
            },
            PoseGroupTemplate {
                name: "rotor_b".into(),
                parent: Some(0),
                module: 1,
                rest: Affine3A::from_translation(Vec3::new(0.0, 30.0, 0.0)),
                motion: Motion::Spin {
                    pivot,
                    axis: Vec3::new(1.0, 0.2, 0.0),
                    rate: -0.7,
                },
            },
        ],
    }
}

/// A two-level hinged chain whose pieces swing through each other's space.
fn bough_asset(rng: &mut Rng) -> CanonicalAssembly {
    CanonicalAssembly {
        name: "bough".into(),
        voxel_size: 1.0 / 32.0,
        palette: vec![
            Material::new([0.0; 3]),
            Material::new([0.3; 3]),
            Material::new([0.1, 0.6, 0.2]),
        ],
        modules: vec![foliage_module(3, rng), foliage_module(3, rng)],
        groups: vec![
            PoseGroupTemplate {
                name: "limb".into(),
                parent: None,
                module: 0,
                rest: Affine3A::IDENTITY,
                motion: Motion::Hinge {
                    pivot: Vec3::new(32.0, 0.0, 32.0),
                    axis: Vec3::Z,
                    min_angle: -0.4,
                    max_angle: 0.4,
                    response_hz: 0.8,
                },
            },
            PoseGroupTemplate {
                name: "twig".into(),
                parent: Some(0),
                module: 1,
                rest: Affine3A::from_rotation_translation(
                    Quat::from_rotation_y(0.9),
                    Vec3::new(20.0, 40.0, 10.0),
                ),
                motion: Motion::Hinge {
                    pivot: Vec3::new(0.0, 0.0, 32.0),
                    axis: Vec3::X,
                    min_angle: -0.6,
                    max_angle: 0.6,
                    response_hz: 1.7,
                },
            },
        ],
    }
}

struct World {
    assemblies: Vec<CanonicalAssembly>,
    templates: Vec<AssetTemplate>,
    geometry: GeometryArenas,
    /// Baked roots per asset, indexed by module.
    roots: Vec<Vec<VoxelRoot>>,
}

impl World {
    fn new(assemblies: Vec<CanonicalAssembly>) -> Self {
        Self::with_hierarchy(assemblies, HierarchyKind::Tree64)
    }

    fn with_hierarchy(assemblies: Vec<CanonicalAssembly>, kind: HierarchyKind) -> Self {
        let mut geometry = GeometryArenas::default();
        let roots = assemblies
            .iter()
            .map(|a| {
                a.modules
                    .iter()
                    .map(|m| geometry.bake(m, kind).unwrap())
                    .collect()
            })
            .collect();
        Self {
            templates: assemblies.iter().map(AssetTemplate::new).collect(),
            assemblies,
            geometry,
            roots,
        }
    }

    fn root(&self, asset: u32, group: u32) -> &VoxelRoot {
        let module = self.assemblies[asset as usize].groups[group as usize].module;
        &self.roots[asset as usize][module as usize]
    }

    fn intersect(&self, snapshot: &SceneSnapshot, ray: &Ray) -> Option<SceneHit> {
        snapshot
            .intersect(
                &self.templates,
                ray,
                0.0,
                f32::MAX,
                |frame, local, t0, t1| {
                    self.root(frame.asset, frame.group)
                        .trace(&self.geometry, local, t0, t1)
                },
            )
            .0
    }

    /// Exhaustive reference: trace every frame, keep the nearest.
    fn intersect_naive(&self, snapshot: &SceneSnapshot, ray: &Ray) -> Option<SceneHit> {
        let mut best: Option<SceneHit> = None;
        for (id, frame) in snapshot.frames.iter().enumerate() {
            let local = ray.transformed(&frame.world_to_group);
            if let Some(voxel) =
                self.root(frame.asset, frame.group)
                    .trace(&self.geometry, &local, 0.0, f32::MAX)
                && best.is_none_or(|b| voxel.t < b.voxel.t)
            {
                best = Some(SceneHit {
                    frame: id as u32,
                    voxel,
                });
            }
        }
        best
    }
}

fn key(hit: Option<SceneHit>) -> Option<(f32, u32, UVec3)> {
    hit.map(|h| (h.voxel.t, h.frame, h.voxel.cell))
}

fn scatter_instances(world: &World, rng: &mut Rng, count: usize, time: f32) -> Vec<InstanceDesc> {
    (0..count)
        .map(|i| {
            let asset = (i % world.assemblies.len()) as u32;
            let assembly = &world.assemblies[asset as usize];
            let rotation = Quat::from_euler(
                glam::EulerRot::YXZ,
                rng.range(0.0, std::f32::consts::TAU),
                rng.range(-0.3, 0.3),
                0.0,
            );
            let position = Vec3::new(
                rng.range(-6.0, 6.0),
                rng.range(-1.0, 1.0),
                rng.range(-6.0, 6.0),
            );
            let scale = assembly.voxel_size * rng.range(0.7, 1.4);
            let driver = PoseDriver {
                phase: rng.range(0.0, std::f32::consts::TAU),
                gust: rng.unit(),
            };
            InstanceDesc {
                key: i as u64,
                asset,
                root_to_world: Affine3A::from_scale_rotation_translation(
                    Vec3::splat(scale),
                    rotation,
                    position,
                ),
                pose: driver.evaluate(assembly, time),
            }
        })
        .collect()
}

fn random_ray(rng: &mut Rng) -> Ray {
    let target = Vec3::new(
        rng.range(-6.0, 6.0),
        rng.range(-1.0, 3.0),
        rng.range(-6.0, 6.0),
    );
    let dir = rng.direction();
    Ray::new(target - dir * 20.0, dir)
}

#[test]
fn every_acceleration_mode_matches_exhaustive_search() {
    for kind in HierarchyKind::ALL {
        let mut rng = Rng(11);
        let world = World::with_hierarchy(vec![rotor_asset(), bough_asset(&mut rng)], kind);

        for step in 0..6 {
            let time = step as f32 * 0.83;
            // hundreds of overlapping, rotated instances
            let instances = scatter_instances(&world, &mut rng, 240, time);
            let snapshots = SceneAcceleration::ALL
                .map(|mode| SceneSnapshot::build(&world.templates, &instances, mode, None));
            for snapshot in &snapshots[1..] {
                assert_eq!(snapshot.frames.len(), snapshots[0].frames.len());
            }

            for _ in 0..400 {
                let ray = random_ray(&mut rng);
                let naive = key(world.intersect_naive(&snapshots[0], &ray));
                for snapshot in &snapshots {
                    assert_eq!(
                        key(world.intersect(snapshot, &ray)),
                        naive,
                        "{:?} {kind:?}, t = {time}",
                        snapshot.mode
                    );
                }
            }
        }
    }
}

/// Refit mini-BVHs bound each instance's current pose, so they never need the template's
/// out-of-envelope fallback and never test more groups than the template does.
#[test]
fn refit_mini_bvhs_are_tighter_than_templates() {
    let mut rng = Rng(13);
    let world = World::new(vec![bough_asset(&mut rng), rotor_asset()]);
    let instances = scatter_instances(&world, &mut rng, 200, 2.1);
    let template = SceneSnapshot::build(
        &world.templates,
        &instances,
        SceneAcceleration::TemplateMiniBvh,
        None,
    );
    let refit = SceneSnapshot::build(
        &world.templates,
        &instances,
        SceneAcceleration::InstanceRefit,
        None,
    );
    assert_eq!(refit.instance_bvhs.len(), instances.len());
    assert!(refit.instances.iter().all(|i| i.refit_bvh.is_some()));

    let (mut template_groups, mut refit_groups) = (0, 0);
    for _ in 0..2000 {
        let ray = random_ray(&mut rng);
        let trace = |snapshot: &SceneSnapshot| {
            snapshot.intersect(
                &world.templates,
                &ray,
                0.0,
                f32::MAX,
                |frame, local, t0, t1| {
                    world
                        .root(frame.asset, frame.group)
                        .trace(&world.geometry, local, t0, t1)
                },
            )
        };
        let (a, a_stats) = trace(&template);
        let (b, b_stats) = trace(&refit);
        assert_eq!(key(a), key(b));
        template_groups += a_stats.group_candidates;
        refit_groups += b_stats.group_candidates;
    }
    assert!(refit_groups <= template_groups);
}

#[test]
fn rotated_pose_groups_stay_hittable() {
    let world = World::new(vec![rotor_asset()]);
    let blade_voxel = UVec3::new(45, 30, 31);
    assert_ne!(world.assemblies[0].modules[1].get(blade_voxel), 0);

    for step in 0..64 {
        let time = step as f32 * 0.21;
        let instances = vec![InstanceDesc {
            key: 0,
            asset: 0,
            root_to_world: Affine3A::from_scale(Vec3::splat(1.0 / 32.0)),
            pose: PoseDriver::default().evaluate(&world.assemblies[0], time),
        }];
        let snapshot = SceneSnapshot::build(
            &world.templates,
            &instances,
            SceneAcceleration::TemplateMiniBvh,
            None,
        );

        // both rotors show the same shared module under different transforms
        for group in [1u32, 2] {
            let frame = snapshot
                .frames
                .iter()
                .position(|f| f.group == group)
                .unwrap() as u32;
            let f = &snapshot.frames[frame as usize];
            let target = f
                .group_to_world
                .transform_point3(blade_voxel.as_vec3() + Vec3::splat(0.5));
            // approach along the voxel's own (rotated) -y axis so nothing in the module occludes it
            let up = f.group_to_world.transform_vector3(Vec3::Y).normalize();
            let ray = Ray::new(target + up * 0.5, -up);
            let hit = world
                .intersect(&snapshot, &ray)
                .expect("rotated group was missed");
            assert_eq!(
                (hit.frame, hit.voxel.cell),
                (frame, blade_voxel),
                "group {group} at t = {time}"
            );
        }
    }
}

#[test]
fn out_of_envelope_poses_fall_back_safely() {
    let mut rng = Rng(12);
    let world = World::new(vec![bough_asset(&mut rng)]);
    let mut pose = PoseState::rest(&world.assemblies[0]);
    // far outside the hinge's legal range, so the template envelopes don't cover it
    pose.angles[1] = 2.5;
    let instances = vec![InstanceDesc {
        key: 0,
        asset: 0,
        root_to_world: Affine3A::from_scale(Vec3::splat(1.0 / 32.0)),
        pose,
    }];
    let snapshot = SceneSnapshot::build(
        &world.templates,
        &instances,
        SceneAcceleration::TemplateMiniBvh,
        None,
    );
    assert!(!snapshot.instances[0].uses_template);
    // refitting needs no fallback: its bounds follow the actual pose
    let refit = SceneSnapshot::build(
        &world.templates,
        &instances,
        SceneAcceleration::InstanceRefit,
        None,
    );
    assert!(refit.instances[0].refit_bvh.is_some());

    let mut hits = 0;
    for _ in 0..1500 {
        let target = Vec3::new(
            rng.range(-1.0, 3.0),
            rng.range(-1.0, 3.0),
            rng.range(-1.0, 3.0),
        );
        let dir = rng.direction();
        let ray = Ray::new(target - dir * 10.0, dir);
        let naive = key(world.intersect_naive(&snapshot, &ray));
        hits += naive.is_some() as u32;
        assert_eq!(key(world.intersect(&snapshot, &ray)), naive);
        assert_eq!(key(world.intersect(&refit, &ray)), naive);
    }
    assert!(hits > 50, "the test rays should actually hit the asset");
}

#[test]
fn far_from_origin_and_nonuniform_instances_hit_their_own_voxels() {
    let world = World::new(vec![rotor_asset()]);
    let base_voxel = UVec3::new(30, 5, 30);
    for (translation, scale) in [
        (
            Vec3::new(10_000.0, -300.0, 4_000.0),
            Vec3::new(1.0, 2.0, 0.5) / 32.0,
        ),
        (
            Vec3::new(-2_500.0, 40.0, 0.0),
            Vec3::new(0.5, 0.5, 3.0) / 32.0,
        ),
    ] {
        let to_world = Affine3A::from_scale_rotation_translation(
            scale,
            Quat::from_rotation_y(0.6),
            translation,
        );
        let instances = vec![InstanceDesc {
            key: 0,
            asset: 0,
            root_to_world: to_world,
            pose: PoseState::rest(&world.assemblies[0]),
        }];
        let snapshot = SceneSnapshot::build(
            &world.templates,
            &instances,
            SceneAcceleration::FlattenedGroups,
            None,
        );
        let target = to_world.transform_point3(base_voxel.as_vec3() + Vec3::new(0.5, 1.0, 0.5));
        let ray = Ray::new(target + Vec3::Y * 0.01, Vec3::NEG_Y);
        let hit = world.intersect(&snapshot, &ray).expect("hit");
        assert_eq!(hit.voxel.cell, base_voxel);
        assert_eq!(snapshot.frames[hit.frame as usize].group, 0);
    }
}

#[test]
fn history_tracks_previous_transforms() {
    let world = World::new(vec![rotor_asset()]);
    let instance = |time: f32| InstanceDesc {
        key: 7,
        asset: 0,
        root_to_world: Affine3A::IDENTITY,
        pose: PoseDriver::default().evaluate(&world.assemblies[0], time),
    };
    let first = SceneSnapshot::build(
        &world.templates,
        &[instance(0.0)],
        SceneAcceleration::TemplateMiniBvh,
        None,
    );
    let second = SceneSnapshot::build(
        &world.templates,
        &[instance(0.5)],
        SceneAcceleration::TemplateMiniBvh,
        Some(&first),
    );
    for (a, b) in first.frames.iter().zip(&second.frames) {
        assert_eq!(b.prev_group_to_world, a.group_to_world);
    }
    assert_ne!(
        second.frames[1].group_to_world,
        second.frames[1].prev_group_to_world
    );
}

/// The gate's reconstruction contract: from a hit's world distance and resolved frame, the world
/// position lands inside the exact reported cell, and its previous position is where that same
/// local point was in the previous snapshot, for moving and static groups alike.
#[test]
fn hits_reconstruct_position_and_motion() {
    let mut rng = Rng(21);
    let world = World::new(vec![rotor_asset(), bough_asset(&mut rng)]);
    let before = scatter_instances(&world, &mut Rng(22), 120, 0.4);
    let after = scatter_instances(&world, &mut Rng(22), 120, 0.55);
    let previous = SceneSnapshot::build(
        &world.templates,
        &before,
        SceneAcceleration::TemplateMiniBvh,
        None,
    );
    let current = SceneSnapshot::build(
        &world.templates,
        &after,
        SceneAcceleration::TemplateMiniBvh,
        Some(&previous),
    );

    let (mut hits, mut moving) = (0, 0);
    for _ in 0..3000 {
        let ray = random_ray(&mut rng);
        let Some(hit) = world.intersect(&current, &ray) else {
            continue;
        };
        hits += 1;
        let frame = &current.frames[hit.frame as usize];
        let p = ray.at(hit.voxel.t);

        // the reconstructed point lies in (or on the boundary of) the reported cell
        let local = frame.world_to_group.transform_point3(p);
        let lo = hit.voxel.cell.as_vec3();
        let slack = 2e-3 * (1.0 + local.abs().max_element());
        assert!(
            local.cmpge(lo - slack).all() && local.cmple(lo + 1.0 + slack).all(),
            "hit at local {local} outside cell {}",
            hit.voxel.cell
        );

        // and its motion follows the same instance's group from the previous snapshot
        let then = &previous.frames[hit.frame as usize];
        assert_eq!((then.instance, then.group), (frame.instance, frame.group));
        let expected = then.group_to_world.transform_point3(local);
        let reconstructed = frame.previous_position(p);
        assert!(
            reconstructed.distance(expected) < 1e-4 * (1.0 + expected.length()),
            "{reconstructed} != {expected}"
        );
        moving += (reconstructed.distance(p) > 1e-4) as u32;
    }
    assert!(hits > 500, "the rays should hit the scene");
    assert!(moving > 50, "some hits should be on animated groups");
}
