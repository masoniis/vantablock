//! Every local hierarchy must agree with brute force on every ray, especially the nasty ones:
//! rays on node and brick boundaries, through exact edges and corners, axis-aligned, backwards,
//! and starting inside geometry. See `voxel::reference` for the acceptance rules. Each test runs
//! against both the 64-tree and the octree.

mod common;

use common::{BOUNDARIES, Rng, boundary_module, foliage_module};
use glam::{UVec3, Vec3};
use voxel::{
    GeometryArenas, HierarchyKind, Ray, VoxelModule, VoxelRoot, hierarchy::tree64,
    lod::FootprintFilter, reference::check_hit,
};

/// Bakes `module` as every hierarchy, each in its own arenas.
fn bake_all(module: &VoxelModule) -> Vec<(GeometryArenas, VoxelRoot)> {
    HierarchyKind::ALL
        .into_iter()
        .map(|kind| {
            let mut arenas = GeometryArenas::default();
            let root = arenas.bake(module, kind).unwrap();
            (arenas, root)
        })
        .collect()
}

/// Checks one ray against every baked hierarchy.
fn assert_ray(module: &VoxelModule, baked: &[(GeometryArenas, VoxelRoot)], ray: Ray, t_min: f32) {
    for (arenas, root) in baked {
        let hit = root.trace(arenas, &ray, t_min, f32::MAX);
        if let Err(e) = check_hit(module, &ray, t_min, f32::MAX, hit.map(|h| (h.t, h.cell))) {
            panic!(
                "{:?}: {e}\nray: {ray:?} t_min {t_min}\nhit: {hit:?}",
                root.kind()
            );
        }
        if let Some(hit) = hit {
            // the reported attribute and terminal index must name the hit cell's own data
            assert_eq!(hit.material, module.get(hit.cell), "{:?}", root.kind());
            let local = hit.cell % 4;
            assert_eq!(hit.terminal_index, local.x + 4 * local.y + 16 * local.z);
        }
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
        let baked = bake_all(&module);
        let extent = 4f32.powi(levels);
        for _ in 0..1500 {
            let dir = rng.direction();
            let target = Vec3::new(
                rng.range(0.0, extent),
                rng.range(0.0, extent),
                rng.range(0.0, extent),
            );
            let origin = target - dir * rng.range(1.0, extent * 1.5);
            assert_ray(&module, &baked, Ray::new(origin, dir), 0.0);
        }
    }
}

#[test]
fn rays_aimed_at_voxel_corners_edges_and_faces() {
    let mut rng = Rng(2);
    let module = boundary_module(5, &mut rng);
    let baked = bake_all(&module);
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
        assert_ray(&module, &baked, Ray::new(origin, dir), 0.0);
    }
}

#[test]
fn rays_running_along_node_boundary_planes() {
    let mut rng = Rng(3);
    let module = boundary_module(5, &mut rng);
    let baked = bake_all(&module);

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
            assert_ray(&module, &baked, Ray::new(origin, dir.normalize()), 0.0);
        }
    }
}

#[test]
fn rays_starting_inside_and_with_t_min() {
    let mut rng = Rng(4);
    let module = foliage_module(4, &mut rng);
    let baked = bake_all(&module);
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
        assert_ray(&module, &baked, Ray::new(origin, rng.direction()), t_min);
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
    for kind in HierarchyKind::ALL {
        let mut arenas = GeometryArenas::default();
        let root = arenas.bake(&module, kind).unwrap();
        let stats = arenas.attributes.stats();
        assert!(stats.dedup_hits > 0, "{kind:?} shared no attribute blocks");

        for tile in 0..16 {
            let base = UVec3::new(16 * (tile % 4), 16 * (tile / 4), 100);
            let target = base + UVec3::new(5, 5, 5);
            let ray = Ray::new(target.as_vec3() + Vec3::new(0.5, 0.5, -50.0), Vec3::Z);
            let hit = root.trace(&arenas, &ray, 0.0, f32::MAX).unwrap();
            assert_eq!(hit.cell, target, "{kind:?} tile {tile}");
        }
    }
}

/// Tiny pages force a multi-page layout; results must not change at all.
#[test]
fn multi_page_layout_traces_identically() {
    let mut rng = Rng(5);
    let module = foliage_module(4, &mut rng);
    for kind in HierarchyKind::ALL {
        let mut single = GeometryArenas::default();
        let single_root = single.bake(&module, kind).unwrap();
        let mut paged = GeometryArenas::with_page_words(256);
        let paged_root = paged.bake(&module, kind).unwrap();

        assert!(single_root.single_page());
        assert!(!paged_root.single_page());
        assert!(paged.topology.pages().len() > 4);

        for _ in 0..2000 {
            let dir = rng.direction();
            let origin = Vec3::splat(128.0) - dir * 300.0
                + Vec3::new(rng.range(-60.0, 60.0), rng.range(-60.0, 60.0), 0.0);
            let ray = Ray::new(origin, dir);
            let a = single_root.trace(&single, &ray, 0.0, f32::MAX);
            let b = paged_root.trace(&paged, &ray, 0.0, f32::MAX);
            assert_eq!(
                a.map(|h| (h.t, h.cell, h.face)),
                b.map(|h| (h.t, h.cell, h.face)),
                "{kind:?}"
            );
        }
    }
}

/// Both hierarchies bake from the same canonical module, so they must agree with each other as
/// well as with the reference: same hit distance on every ray, and the same cell whenever the
/// nearest voxel is unique.
#[test]
fn hierarchies_agree_with_each_other() {
    let mut rng = Rng(6);
    let module = foliage_module(5, &mut rng);
    let baked = bake_all(&module);
    let mut voxels: Vec<UVec3> = module.voxels().map(|(p, _)| p).collect();
    voxels.sort_unstable_by_key(|p| (p.z, p.y, p.x));
    let (mut hits, mut same_cell) = (0, 0);
    for _ in 0..3000 {
        let dir = rng.direction();
        let target = voxels[rng.below(voxels.len() as u32) as usize].as_vec3()
            + Vec3::new(rng.unit(), rng.unit(), rng.unit());
        let ray = Ray::new(target - dir * rng.range(10.0, 1500.0), dir);
        let [a, b] = [0, 1].map(|i| baked[i].1.trace(&baked[i].0, &ray, 0.0, f32::MAX));
        assert_eq!(a.map(|h| h.t), b.map(|h| h.t), "ray {ray:?}");
        hits += a.is_some() as u32;
        same_cell += (a.is_some() && a.map(|h| h.cell) == b.map(|h| h.cell)) as u32;
    }
    assert!(hits > 2000, "the test rays should hit the module");
    assert_eq!(
        same_cell, hits,
        "random rays almost never tie, so cells should match"
    );
}

/// Footprint filtering is exact with a zero-width cone, and with a wide cone every filtered hit
/// names an aligned, occupied region at the distance the ray enters it.
#[test]
fn footprint_filtering_reports_aligned_occupied_regions() {
    let mut rng = Rng(7);
    let module = foliage_module(4, &mut rng);
    let mut arenas = GeometryArenas::default();
    let (root, _) = arenas.bake_tree64(&module).unwrap();
    let extent = root.extent() as f32;
    let exact = FootprintFilter {
        radius: 0.0,
        spread: 0.0,
        threshold: 0.5,
    };
    let wide = FootprintFilter {
        radius: 2.0,
        spread: 0.02,
        threshold: 0.1,
    };
    let mut filtered = 0;
    for _ in 0..1500 {
        let dir = rng.direction();
        let target = Vec3::new(
            rng.range(0.0, extent),
            rng.range(0.0, extent),
            rng.range(0.0, extent),
        );
        let ray = Ray::new(target - dir * rng.range(1.0, extent * 1.5), dir);
        let plain = tree64::trace(&arenas, &root, &ray, 0.0, f32::MAX);
        let (hit, _) = tree64::trace_filtered(&arenas, &root, &ray, 0.0, f32::MAX, &exact);
        assert_eq!(hit, plain, "ray {ray:?}");

        let Some(hit) = tree64::trace_filtered(&arenas, &root, &ray, 0.0, f32::MAX, &wide).0 else {
            continue;
        };
        if hit.lod == 0 {
            continue;
        }
        filtered += 1;
        let size = 4u32.pow(hit.lod);
        assert_eq!(hit.cell % size, UVec3::ZERO, "ray {ray:?}");
        // the hit is the entry of the node's box clipped to the occupied bounds
        let lo = hit.cell.as_vec3().max(root.bounds.min);
        let hi = (hit.cell.as_vec3() + Vec3::splat(size as f32)).min(root.bounds.max);
        let (te, tx) = voxel::math::slab(lo, hi, &ray);
        assert!(
            te <= tx && (hit.t - te.max(0.0)).abs() < 1e-3,
            "ray {ray:?}"
        );
        let occupied = (0..size.pow(3)).any(|i| {
            module.get(hit.cell + UVec3::new(i % size, i / size % size, i / size / size)) != 0
        });
        assert!(occupied, "ray {ray:?} hit an empty region");
    }
    assert!(
        filtered > 50,
        "the wide cone should filter ({filtered} filtered hits)"
    );
}
