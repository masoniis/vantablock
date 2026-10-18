//! Canonical assemblies: a logical asset built from voxel modules and articulated pose groups.
//!
//! Pose groups form an *animation parent tree*: each group has a parent-relative rest transform
//! and a motion domain. Several groups may reference the same module, sharing one immutable voxel
//! root while getting distinct runtime frames. Spatial search over groups (the template mini-BVH)
//! is derived separately and need not follow this parent tree.
//!
//! All group-local coordinates are in voxels of the group's module. The asset root frame is the
//! root group's parent space, also in voxels; `voxel_size` converts asset units to meters. By
//! convention the root frame's origin is the asset's ground anchor (the center of its base) with
//! +Y up, so placing an asset means saying where its base sits.

use crate::{math::Aabb, module::VoxelModule, pose::PoseState};
use glam::{Affine3A, Quat, Vec3};
use std::fmt;

/// A palette entry. Material ID `0` (empty) still occupies slot 0 so IDs index directly.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Material {
    /// Linear RGB albedo.
    pub albedo: [f32; 3],
    pub roughness: f32,
    /// Emitted radiance as a multiple of albedo.
    pub emission: f32,
}

impl Material {
    pub const fn new(albedo: [f32; 3]) -> Self {
        Self {
            albedo,
            roughness: 0.8,
            emission: 0.0,
        }
    }
}

/// The motion domain of a pose group: every state it may legally take.
///
/// Rotations happen in group-local space about `pivot`, before the group's rest transform.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Motion {
    Static,
    /// Bounded rotation, e.g. a bough bending in the wind.
    Hinge {
        pivot: Vec3,
        axis: Vec3,
        min_angle: f32,
        max_angle: f32,
        /// Natural response frequency, so a shared wind driver moves groups differently.
        response_hz: f32,
    },
    /// Unbounded rotation, e.g. a machine rotor.
    Spin {
        pivot: Vec3,
        axis: Vec3,
        rate: f32,
    },
}

impl Motion {
    /// The group's local transform at `angle` (ignored for static groups).
    pub fn local_transform(&self, angle: f32) -> Affine3A {
        match *self {
            Motion::Static => Affine3A::IDENTITY,
            Motion::Hinge { pivot, axis, .. } | Motion::Spin { pivot, axis, .. } => {
                Affine3A::from_translation(pivot)
                    * Affine3A::from_quat(Quat::from_axis_angle(axis.normalize(), angle))
                    * Affine3A::from_translation(-pivot)
            }
        }
    }

    /// Whether `angle` is a legal state of this motion domain.
    pub fn allows(&self, angle: f32) -> bool {
        match *self {
            Motion::Static => angle == 0.0,
            Motion::Hinge {
                min_angle,
                max_angle,
                ..
            } => (min_angle..=max_angle).contains(&angle),
            Motion::Spin { .. } => angle.is_finite(),
        }
    }

    /// A conservative bound of `bounds` under every legal state of this motion.
    ///
    /// Rotations are enclosed by the sphere around the pivot reaching the farthest corner. This is
    /// loose for small hinge ranges, but always safe; tighter bounds are a derived optimization.
    pub fn sweep(&self, bounds: Aabb) -> Aabb {
        match *self {
            Motion::Static => bounds,
            Motion::Hinge { pivot, .. } | Motion::Spin { pivot, .. } => {
                if bounds.is_empty() {
                    return bounds;
                }
                let radius = bounds
                    .corners()
                    .iter()
                    .map(|c| c.distance(pivot))
                    .fold(0.0, f32::max);
                Aabb::sphere(pivot, radius).padded()
            }
        }
    }
}

/// One articulated, rigidly moving part of an asset.
#[derive(Clone, Debug, PartialEq)]
pub struct PoseGroupTemplate {
    pub name: String,
    /// The animation parent. Parents always precede their children.
    pub parent: Option<u16>,
    /// The module whose voxels this group shows.
    pub module: u16,
    /// Maps group-local voxels into the parent group's space (or the asset root).
    pub rest: Affine3A,
    pub motion: Motion,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AssemblyError {
    NoGroups,
    ParentNotBefore { group: usize },
    MissingModule { group: usize, module: u16 },
    MissingMaterial { module: usize, material: u16 },
    NonInvertibleRest { group: usize },
}

impl fmt::Display for AssemblyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoGroups => write!(f, "assembly has no pose groups"),
            Self::ParentNotBefore { group } => {
                write!(f, "group {group}'s parent does not precede it")
            }
            Self::MissingModule { group, module } => {
                write!(f, "group {group} references missing module {module}")
            }
            Self::MissingMaterial { module, material } => {
                write!(
                    f,
                    "module {module} uses material {material} outside the palette"
                )
            }
            Self::NonInvertibleRest { group } => {
                write!(f, "group {group} has a non-invertible rest transform")
            }
        }
    }
}

impl std::error::Error for AssemblyError {}

/// A logical asset: palette, canonical modules, and pose groups. See the [module docs](self).
#[derive(Clone, Debug, PartialEq)]
pub struct CanonicalAssembly {
    pub name: String,
    /// Meters per voxel in the asset root frame.
    pub voxel_size: f32,
    pub palette: Vec<Material>,
    pub modules: Vec<VoxelModule>,
    pub groups: Vec<PoseGroupTemplate>,
}

impl CanonicalAssembly {
    pub fn validate(&self) -> Result<(), AssemblyError> {
        if self.groups.is_empty() {
            return Err(AssemblyError::NoGroups);
        }
        for (i, group) in self.groups.iter().enumerate() {
            if group.parent.is_some_and(|p| p as usize >= i) {
                return Err(AssemblyError::ParentNotBefore { group: i });
            }
            if group.module as usize >= self.modules.len() {
                return Err(AssemblyError::MissingModule {
                    group: i,
                    module: group.module,
                });
            }
            if group.rest.matrix3.determinant().abs() < 1e-12 {
                return Err(AssemblyError::NonInvertibleRest { group: i });
            }
        }
        for (i, module) in self.modules.iter().enumerate() {
            if let Some((_, material)) = module
                .voxels()
                .find(|&(_, m)| m as usize >= self.palette.len())
            {
                return Err(AssemblyError::MissingMaterial {
                    module: i,
                    material,
                });
            }
        }
        Ok(())
    }

    /// Whether spatial search needs an object-internal mini-BVH. A single-group asset traces
    /// its one voxel root directly.
    pub fn needs_mini_bvh(&self) -> bool {
        self.groups.len() > 1
    }

    /// The occupied bounds of a module in its own voxel space.
    pub fn module_bounds(&self, module: u16) -> Aabb {
        match self.modules[module as usize].occupied_bounds() {
            Some((min, max)) => Aabb::new(min.as_vec3(), max.as_vec3()),
            None => Aabb::EMPTY,
        }
    }

    /// Each group's transform into the asset root frame for `state`.
    pub fn group_to_root(&self, state: &PoseState) -> Vec<Affine3A> {
        group_to_root(&self.groups, state)
    }

    /// The tight current bound of each group in the asset root frame for `state`.
    pub fn current_bounds(&self, state: &PoseState) -> Vec<Aabb> {
        self.group_to_root(state)
            .iter()
            .zip(&self.groups)
            .map(|(m, g)| self.module_bounds(g.module).transformed(m))
            .collect()
    }

    /// A conservative bound of each group, in the asset root frame, enclosing every legal state of
    /// the group and all of its ancestors. These feed the template-shared mini-BVH.
    pub fn motion_envelopes(&self) -> Vec<Aabb> {
        (0..self.groups.len())
            .map(|i| {
                let group = &self.groups[i];
                let mut bounds = group
                    .motion
                    .sweep(self.module_bounds(group.module))
                    .transformed(&group.rest);
                let mut parent = group.parent;
                while let Some(p) = parent {
                    let ancestor = &self.groups[p as usize];
                    bounds = ancestor.motion.sweep(bounds).transformed(&ancestor.rest);
                    parent = ancestor.parent;
                }
                bounds
            })
            .collect()
    }

    /// The bound of the whole asset under every legal state.
    pub fn root_envelope(&self) -> Aabb {
        self.motion_envelopes()
            .iter()
            .fold(Aabb::EMPTY, |acc, b| acc.union(b))
    }
}

/// Resolves every group's transform into the asset root frame by walking the parent tree.
pub fn group_to_root(groups: &[PoseGroupTemplate], state: &PoseState) -> Vec<Affine3A> {
    let mut out: Vec<Affine3A> = Vec::with_capacity(groups.len());
    for (i, group) in groups.iter().enumerate() {
        let local = group.rest * group.motion.local_transform(state.angle(i));
        let to_root = match group.parent {
            Some(p) => out[p as usize] * local,
            None => local,
        };
        out.push(to_root);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{module::VoxelModule, pose::PoseDriver};
    use glam::UVec3;

    fn bar_module() -> VoxelModule {
        let mut m = VoxelModule::new(2);
        for x in 0..12 {
            m.set(UVec3::new(x, 1, 1), 1);
        }
        m
    }

    /// A two-level chain: a spinning arm carrying a hinged arm.
    fn chain() -> CanonicalAssembly {
        CanonicalAssembly {
            name: "chain".into(),
            voxel_size: 0.1,
            palette: vec![Material::new([0.0; 3]), Material::new([1.0; 3])],
            modules: vec![bar_module()],
            groups: vec![
                PoseGroupTemplate {
                    name: "arm".into(),
                    parent: None,
                    module: 0,
                    rest: Affine3A::IDENTITY,
                    motion: Motion::Spin {
                        pivot: Vec3::new(0.0, 1.5, 1.5),
                        axis: Vec3::Y,
                        rate: 1.0,
                    },
                },
                PoseGroupTemplate {
                    name: "forearm".into(),
                    parent: Some(0),
                    module: 0,
                    rest: Affine3A::from_translation(Vec3::new(12.0, 0.0, 0.0)),
                    motion: Motion::Hinge {
                        pivot: Vec3::new(0.0, 1.5, 1.5),
                        axis: Vec3::Z,
                        min_angle: -0.5,
                        max_angle: 0.5,
                        response_hz: 1.0,
                    },
                },
            ],
        }
    }

    #[test]
    fn validation_catches_bad_parents() {
        let mut a = chain();
        assert!(a.validate().is_ok());
        a.groups[0].parent = Some(1);
        assert_eq!(
            a.validate(),
            Err(AssemblyError::ParentNotBefore { group: 0 })
        );
    }

    #[test]
    fn envelopes_contain_every_sampled_legal_state() {
        let a = chain();
        let envelopes = a.motion_envelopes();
        let driver = PoseDriver {
            phase: 0.3,
            gust: 1.0,
        };
        for step in 0..500 {
            let state = driver.evaluate(&a, step as f32 * 0.037);
            assert!(state.is_within_envelope(&a));
            for (current, envelope) in a.current_bounds(&state).iter().zip(&envelopes) {
                assert!(
                    envelope.contains_aabb(current),
                    "{current:?} escaped {envelope:?}"
                );
            }
        }
    }

    #[test]
    fn children_inherit_parent_motion() {
        let a = chain();
        let rest = a.group_to_root(&PoseState::rest(&a));
        let mut spun = PoseState::rest(&a);
        spun.angles[0] = std::f32::consts::FRAC_PI_2;
        let moved = a.group_to_root(&spun);
        // the forearm's origin swings around the arm's pivot with it
        let before = rest[1].transform_point3(Vec3::ZERO);
        let after = moved[1].transform_point3(Vec3::ZERO);
        assert!(before.distance(after) > 5.0);
    }
}
