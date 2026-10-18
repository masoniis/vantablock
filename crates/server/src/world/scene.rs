//! Populates the world from a scene layout: hand-placed objects, scattered groves, and tiled
//! ground. Every object is spawned once as a replicated, server-owned entity.

use bevy::prelude::*;
use lightyear::prelude::*;
use rand::{Rng, SeedableRng, rngs::StdRng};
use serde::Deserialize;
use shared::{
    lifecycle::PersistentPathsResource,
    world::{ObjectAnimation, ObjectPlacement, VantaAssetLibrary, WorldObject},
};
use std::f32::consts::TAU;

/// The scene the server populates on boot, relative to the assets directory.
pub const SCENE_PATH: &str = "server/scenes/showcase.ron";

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
pub struct SceneLayout {
    pub objects: Vec<PlacedObject>,
    pub scatters: Vec<Scatter>,
    pub grids: Vec<TileGrid>,
}

#[derive(Deserialize, Debug)]
pub struct PlacedObject {
    pub asset: String,
    pub position: [f32; 3],
    #[serde(default)]
    pub yaw_degrees: f32,
    #[serde(default = "one")]
    pub scale: f32,
    #[serde(default = "one")]
    pub gust: f32,
}

/// Randomly scatters many rotated, overlapping instances of one asset.
#[derive(Deserialize, Debug)]
pub struct Scatter {
    pub asset: String,
    pub count: u32,
    pub center: [f32; 3],
    pub radius: f32,
    #[serde(default)]
    pub min_spacing: f32,
    #[serde(default = "unit_range")]
    pub scale: (f32, f32),
    /// Maximum random tilt away from upright, for rocks and crystals.
    #[serde(default)]
    pub tilt_degrees: f32,
    #[serde(default = "unit_range")]
    pub gust: (f32, f32),
    #[serde(default)]
    pub seed: u64,
}

/// Tiles an asset over a square grid with random quarter-turn rotations to hide repetition.
#[derive(Deserialize, Debug)]
pub struct TileGrid {
    pub asset: String,
    pub center: [f32; 3],
    pub tiles_per_side: u32,
    /// Distance between tile centers in meters.
    pub spacing: f32,
    #[serde(default)]
    pub seed: u64,
}

fn one() -> f32 {
    1.0
}

fn unit_range() -> (f32, f32) {
    (1.0, 1.0)
}

/// One object to spawn, resolved from the layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub object: WorldObject,
    pub placement: ObjectPlacement,
    pub animation: ObjectAnimation,
}

impl SceneLayout {
    /// Expands the layout into concrete placements. Unknown assets are skipped with a warning.
    pub fn resolve(&self, library: &VantaAssetLibrary) -> Vec<Placement> {
        let mut out = Vec::new();
        let lookup = |name: &str| {
            let id = library.id_by_name(name);
            if id.is_none() {
                warn!("Scene references unknown asset '{}'", name);
            }
            id
        };

        for (i, o) in self.objects.iter().enumerate() {
            let Some(asset) = lookup(&o.asset) else {
                continue;
            };
            out.push(Placement {
                object: WorldObject { asset },
                placement: ObjectPlacement {
                    translation: Vec3::from(o.position),
                    rotation: Quat::from_rotation_y(o.yaw_degrees.to_radians()),
                    scale: o.scale,
                },
                animation: ObjectAnimation {
                    phase: i as f32 * 1.7,
                    gust: o.gust,
                },
            });
        }

        for s in &self.scatters {
            let Some(asset) = lookup(&s.asset) else {
                continue;
            };
            let mut rng = StdRng::seed_from_u64(s.seed);
            let mut placed: Vec<Vec3> = Vec::new();
            for _ in 0..s.count {
                // rejection sample for spacing, giving up gracefully in crowded scatters
                let Some(position) = (0..32).find_map(|_| {
                    let angle = rng.random_range(0.0..TAU);
                    let r = s.radius * rng.random::<f32>().sqrt();
                    let p = Vec3::from(s.center) + Vec3::new(angle.cos() * r, 0.0, angle.sin() * r);
                    placed
                        .iter()
                        .all(|q| q.distance(p) >= s.min_spacing)
                        .then_some(p)
                }) else {
                    continue;
                };
                placed.push(position);
                let tilt = s.tilt_degrees.to_radians();
                let rotation = Quat::from_rotation_y(rng.random_range(0.0..TAU))
                    * Quat::from_rotation_x(rng.random_range(-tilt..=tilt))
                    * Quat::from_rotation_z(rng.random_range(-tilt..=tilt));
                out.push(Placement {
                    object: WorldObject { asset },
                    placement: ObjectPlacement {
                        translation: position,
                        rotation,
                        scale: rng.random_range(s.scale.0..=s.scale.1),
                    },
                    animation: ObjectAnimation {
                        phase: rng.random_range(0.0..TAU),
                        gust: rng.random_range(s.gust.0..=s.gust.1),
                    },
                });
            }
        }

        for g in &self.grids {
            let Some(asset) = lookup(&g.asset) else {
                continue;
            };
            let mut rng = StdRng::seed_from_u64(g.seed);
            let half = (g.tiles_per_side as f32 - 1.0) * 0.5;
            for z in 0..g.tiles_per_side {
                for x in 0..g.tiles_per_side {
                    let offset = Vec3::new(x as f32 - half, 0.0, z as f32 - half) * g.spacing;
                    out.push(Placement {
                        object: WorldObject { asset },
                        placement: ObjectPlacement {
                            translation: Vec3::from(g.center) + offset,
                            rotation: Quat::from_rotation_y(
                                rng.random_range(0..4) as f32 * TAU / 4.0,
                            ),
                            scale: 1.0,
                        },
                        animation: ObjectAnimation::default(),
                    });
                }
            }
        }
        out
    }
}

pub fn load_layout(paths: &PersistentPathsResource) -> SceneLayout {
    let path = paths.assets_dir.join(SCENE_PATH);
    match std::fs::read_to_string(&path).map(|text| ron::from_str::<SceneLayout>(&text)) {
        Ok(Ok(layout)) => layout,
        Ok(Err(e)) => {
            error!("Failed to parse scene {:?}: {}", path, e);
            SceneLayout::default()
        }
        Err(e) => {
            error!("Failed to read scene {:?}: {}", path, e);
            SceneLayout::default()
        }
    }
}

/// Spawns the scene's objects as replicated entities once the server is running.
pub fn spawn_scene_system(
    mut commands: Commands,
    paths: Res<PersistentPathsResource>,
    library: Option<Res<VantaAssetLibrary>>,
) {
    let Some(library) = library else {
        error!("No asset library loaded; the world will be empty");
        return;
    };
    let placements = load_layout(&paths).resolve(&library);
    info!("Spawning {} world objects", placements.len());
    for p in placements {
        commands.spawn((
            p.object,
            p.placement,
            p.animation,
            Replicate::to_clients(NetworkTarget::All),
        ));
    }
}
