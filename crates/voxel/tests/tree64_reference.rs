//! The 64-tree must agree with brute force on every ray, especially the nasty ones: rays on
//! node boundaries, through exact edges and corners, axis-aligned, backwards, and starting
//! inside geometry. See `voxel::reference` for the acceptance rules.

mod common;

use common::{BOUNDARIES, Rng, boundary_module, foliage_module};
use glam::{UVec3, Vec3};
use voxel::{
    GeometryArenas, Ray, VoxelModule,
    hierarchy::tree64::{Tree64Root, trace},
    reference::check_hit,
};

fn bake(module: &VoxelModule) -> (GeometryArenas, Tree64Root) {
    let mut arenas = GeometryArenas::default();
    let (root, _) = arenas.bake_tree64(module).unwrap();
    (arenas, root)
}

fn assert_ray(
    module: &VoxelModule,
    arenas: &GeometryArenas,
    root: &Tree64Root,
    ray: Ray,
    t_min: f32,
) {
    let hit = trace(arenas, root, &ray, t_min, f32::MAX);
    if let Err(e) = check_hit(module, &ray, t_min, f32::MAX, hit.map(|h| (h.t, h.cell))) {
        panic!("{e}\nray: {ray:?} t_min {t_min}\nhit: {hit:?}");
    }
    if let Some(hit) = hit {
        // the reported attribute and terminal index must name the hit cell's own data
        assert_eq!(hit.material, module.get(hit.cell));
        let local = hit.cell % 4;
        assert_eq!(hit.terminal_index, local.x + 4 * local.y + 16 * local.z);
    }
}

#[test]
fn random_rays_match_brute_force() {
    let mut rng = Rng(1);
    for (levels, module) in [
        (3, foliage_module(3, &mut rng)),
        (4, foliage_module(4, &mut rng)),
        (5, boundary_module(5, &mut rng)),
    ] {
        let (arenas, root) = bake(&module);
        let extent = 4f32.powi(levels);
        for _ in 0..1500 {
            let dir = rng.direction();
            let target = Vec3::new(
                rng.range(0.0, extent),
                rng.range(0.0, extent),
                rng.range(0.0, extent),
            );
            let origin = target - dir * rng.range(1.0, extent * 1.5);
            assert_ray(&module, &arenas, &root, Ray::new(origin, dir), 0.0);
        }
    }
}

#[test]
fn rays_aimed_at_voxel_corners_edges_and_faces() {
    let mut rng = Rng(2);
    let module = boundary_module(5, &mut rng);
    let (arenas, root) = bake(&module);
    // sorted, since module iteration order is not deterministic
    let mut voxels: Vec<UVec3> = module.voxels().map(|(p, _)| p).collect();
    voxels.sort_unstable_by_key(|p| (p.z, p.y, p.x));

    for _ in 0..6000 {
        let cell = voxels[rng.below(voxels.len() as u32) as usize].as_vec3();
        // exact corners, edge midpoints, face centers and centers of the target voxel
        let offset = Vec3::new(
            [0.0, 0.5, 1.0][rng.below(3) as usize],
            [0.0, 0.5, 1.0][rng.below(3) as usize],
            [0.0, 0.5, 1.0][rng.below(3) as usize],
        );
        let target = cell + offset;
        let dir = match rng.below(4) {
            // exact diagonals produce simultaneous boundary crossings
            0 => Vec3::new(
                [1.0, -1.0][rng.below(2) as usize],
                [1.0, -1.0][rng.below(2) as usize],
                [1.0, -1.0][rng.below(2) as usize],
            )
            .normalize(),
            // axis-aligned rays graze faces and edges
            1 => [
                Vec3::X,
                Vec3::NEG_X,
                Vec3::Y,
                Vec3::NEG_Y,
                Vec3::Z,
                Vec3::NEG_Z,
            ][rng.below(6) as usize],
            // nearly parallel to a plane: regression for entry cells chosen by flooring a
            // reconstructed position, which rounded into the wrong cell
            2 => {
                let mut d = rng.direction();
                d[rng.below(3) as usize] *= 1e-4;
                d.normalize()
            }
            _ => rng.direction(),
        };
        let origin = target - dir * rng.range(2.0, 1500.0);
        assert_ray(&module, &arenas, &root, Ray::new(origin, dir), 0.0);
    }
}

#[test]
fn rays_running_along_node_boundary_planes() {
    let mut rng = Rng(3);
    let module = boundary_module(5, &mut rng);
    let (arenas, root) = bake(&module);

    for &plane in &BOUNDARIES {
        for _ in 0..200 {
            // origin exactly on an integer plane, moving within it
            let mut origin = Vec3::new(rng.range(-10.0, 1030.0), rng.range(-10.0, 1030.0), -5.0);
            let axis = rng.below(3) as usize;
            origin[axis] = plane as f32;
            let mut dir = rng.direction();
            dir[axis] = 0.0;
            if dir.length_squared() < 1e-6 {
                continue;
            }
            assert_ray(
                &module,
                &arenas,
                &root,
                Ray::new(origin, dir.normalize()),
                0.0,
            );
        }
    }
}

#[test]
fn rays_starting_inside_and_with_t_min() {
    let mut rng = Rng(4);
    let module = foliage_module(4, &mut rng);
    let (arenas, root) = bake(&module);
    for _ in 0..2000 {
        let origin = Vec3::new(
            rng.range(0.0, 256.0),
            rng.range(0.0, 256.0),
            rng.range(0.0, 256.0),
        );
        let t_min = if rng.below(2) == 0 {
            0.0
        } else {
            rng.range(0.0, 40.0)
        };
        assert_ray(
            &module,
            &arenas,
            &root,
            Ray::new(origin, rng.direction()),
            t_min,
        );
    }
}

/// Repeated identical bricks share storage, but every occurrence must still report its own
/// spatial cell rather than the shared data's.
#[test]
fn shared_subtrees_keep_distinct_spatial_identity() {
    let mut module = VoxelModule::new(4);
    for tile in 0..16 {
        let base = UVec3::new(16 * (tile % 4), 16 * (tile / 4), 100);
        for (x, y, z) in [(1, 1, 1), (2, 3, 1), (5, 5, 5), (9, 2, 14)] {
            module.set(base + UVec3::new(x, y, z), 3);
        }
    }
    let mut arenas = GeometryArenas::default();
    let (root, stats) = arenas.bake_tree64(&module).unwrap();
    assert!(arenas.topology.stats().dedup_hits > 0 || arenas.attributes.stats().dedup_hits > 0);
    assert!(stats.terminal_nodes > arenas.attributes.stats().blocks);

    for tile in 0..16 {
        let base = UVec3::new(16 * (tile % 4), 16 * (tile / 4), 100);
        let target = base + UVec3::new(5, 5, 5);
        let ray = Ray::new(target.as_vec3() + Vec3::new(0.5, 0.5, -50.0), Vec3::Z);
        let hit = trace(&arenas, &root, &ray, 0.0, f32::MAX).unwrap();
        assert_eq!(hit.cell, target, "tile {tile}");
    }
}

/// Tiny pages force a multi-page layout; results must not change at all.
#[test]
fn multi_page_layout_traces_identically() {
    let mut rng = Rng(5);
    let module = foliage_module(4, &mut rng);
    let (single, single_root) = bake(&module);
    let mut paged = GeometryArenas::with_page_words(256);
    let (paged_root, _) = paged.bake_tree64(&module).unwrap();

    assert!(single_root.single_page);
    assert!(!paged_root.single_page);
    assert!(paged.topology.pages().len() > 4);

    for _ in 0..2000 {
        let dir = rng.direction();
        let origin = Vec3::splat(128.0) - dir * 300.0
            + Vec3::new(rng.range(-60.0, 60.0), rng.range(-60.0, 60.0), 0.0);
        let ray = Ray::new(origin, dir);
        let a = trace(&single, &single_root, &ray, 0.0, f32::MAX);
        let b = trace(&paged, &paged_root, &ray, 0.0, f32::MAX);
        assert_eq!(
            a.map(|h| (h.t, h.cell, h.face)),
            b.map(|h| (h.t, h.cell, h.face))
        );
    }
}
