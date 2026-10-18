//! The `.vanta` asset file format.
//!
//! A `.vanta` file stores only canonical data: the palette, voxel modules (as non-empty bricks),
//! and the pose-group assembly. Hierarchies, LOD summaries, and BVHs are always rebuilt from it.
//!
//! Layout: the magic `VNTA`, a little-endian `u32` format version, then a zstd-compressed body of
//! little-endian fields in the order written by [`write`].

use crate::{
    assembly::{CanonicalAssembly, Material, Motion, PoseGroupTemplate},
    module::{BRICK_VOXELS, Brick, VoxelModule},
};
use glam::{Affine3A, UVec3, Vec3};
use std::{fmt, io};

pub const MAGIC: &[u8; 4] = b"VNTA";
pub const VERSION: u32 = 1;
const COMPRESSION_LEVEL: i32 = 9;

#[derive(Debug)]
pub enum FormatError {
    Io(io::Error),
    BadMagic,
    UnsupportedVersion(u32),
    Corrupt(&'static str),
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io error: {e}"),
            Self::BadMagic => write!(f, "not a .vanta file"),
            Self::UnsupportedVersion(v) => write!(f, "unsupported .vanta version {v}"),
            Self::Corrupt(what) => write!(f, "corrupt .vanta file: {what}"),
        }
    }
}

impl std::error::Error for FormatError {}

impl From<io::Error> for FormatError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

/// Encodes an assembly as `.vanta` bytes.
pub fn write(assembly: &CanonicalAssembly) -> Result<Vec<u8>, FormatError> {
    let mut w = Writer::default();
    w.string(&assembly.name);
    w.f32(assembly.voxel_size);

    w.u32(assembly.palette.len() as u32);
    for m in &assembly.palette {
        m.albedo.iter().for_each(|&c| w.f32(c));
        w.f32(m.roughness);
        w.f32(m.emission);
    }

    w.u32(assembly.modules.len() as u32);
    for module in &assembly.modules {
        w.u8(module.levels());
        // sorted so identical assets always produce identical files
        let mut bricks: Vec<_> = module.bricks().iter().collect();
        bricks.sort_unstable_by_key(|(c, _)| (c.z, c.y, c.x));
        w.u32(bricks.len() as u32);
        for (coord, brick) in bricks {
            coord.to_array().iter().for_each(|&c| w.u32(c));
            brick.materials.iter().for_each(|&m| w.u16(m));
        }
    }

    w.u32(assembly.groups.len() as u32);
    for g in &assembly.groups {
        w.string(&g.name);
        w.u16(g.parent.unwrap_or(u16::MAX));
        w.u16(g.module);
        g.rest.to_cols_array().iter().for_each(|&c| w.f32(c));
        match g.motion {
            Motion::Static => w.u8(0),
            Motion::Hinge {
                pivot,
                axis,
                min_angle,
                max_angle,
                response_hz,
            } => {
                w.u8(1);
                w.vec3(pivot);
                w.vec3(axis);
                w.f32(min_angle);
                w.f32(max_angle);
                w.f32(response_hz);
            }
            Motion::Spin { pivot, axis, rate } => {
                w.u8(2);
                w.vec3(pivot);
                w.vec3(axis);
                w.f32(rate);
            }
        }
    }

    let mut out = Vec::with_capacity(w.bytes.len() / 4);
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&zstd::encode_all(&w.bytes[..], COMPRESSION_LEVEL)?);
    Ok(out)
}

/// Decodes `.vanta` bytes.
pub fn read(bytes: &[u8]) -> Result<CanonicalAssembly, FormatError> {
    if bytes.len() < 8 || &bytes[..4] != MAGIC {
        return Err(FormatError::BadMagic);
    }
    let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    if version != VERSION {
        return Err(FormatError::UnsupportedVersion(version));
    }
    let body = zstd::decode_all(&bytes[8..])?;
    let mut r = Reader { bytes: &body };

    let name = r.string()?;
    let voxel_size = r.f32()?;

    let palette = (0..r.u32()?)
        .map(|_| {
            Ok(Material {
                albedo: [r.f32()?, r.f32()?, r.f32()?],
                roughness: r.f32()?,
                emission: r.f32()?,
            })
        })
        .collect::<Result<Vec<_>, FormatError>>()?;

    let mut modules = Vec::new();
    for _ in 0..r.u32()? {
        let levels = r.u8()?;
        if !(1..=crate::module::MAX_LEVELS).contains(&levels) {
            return Err(FormatError::Corrupt("module levels"));
        }
        let mut module = VoxelModule::new(levels);
        for _ in 0..r.u32()? {
            let coord = UVec3::new(r.u32()?, r.u32()?, r.u32()?);
            if (coord * 4).max_element() >= module.extent() {
                return Err(FormatError::Corrupt("brick outside module"));
            }
            let mut brick = Brick::default();
            for i in 0..BRICK_VOXELS {
                brick.materials[i] = r.u16()?;
            }
            module.insert_brick(coord, brick);
        }
        modules.push(module);
    }

    let mut groups = Vec::new();
    for _ in 0..r.u32()? {
        let name = r.string()?;
        let parent = r.u16()?;
        let module = r.u16()?;
        let mut cols = [0f32; 12];
        for c in &mut cols {
            *c = r.f32()?;
        }
        let motion = match r.u8()? {
            0 => Motion::Static,
            1 => Motion::Hinge {
                pivot: r.vec3()?,
                axis: r.vec3()?,
                min_angle: r.f32()?,
                max_angle: r.f32()?,
                response_hz: r.f32()?,
            },
            2 => Motion::Spin {
                pivot: r.vec3()?,
                axis: r.vec3()?,
                rate: r.f32()?,
            },
            _ => return Err(FormatError::Corrupt("motion tag")),
        };
        groups.push(PoseGroupTemplate {
            name,
            parent: (parent != u16::MAX).then_some(parent),
            module,
            rest: Affine3A::from_cols_array(&cols),
            motion,
        });
    }

    let assembly = CanonicalAssembly {
        name,
        voxel_size,
        palette,
        modules,
        groups,
    };
    assembly
        .validate()
        .map_err(|_| FormatError::Corrupt("assembly failed validation"))?;
    Ok(assembly)
}

#[derive(Default)]
struct Writer {
    bytes: Vec<u8>,
}

impl Writer {
    fn u8(&mut self, v: u8) {
        self.bytes.push(v);
    }
    fn u16(&mut self, v: u16) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }
    fn u32(&mut self, v: u32) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }
    fn f32(&mut self, v: f32) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }
    fn vec3(&mut self, v: Vec3) {
        v.to_array().iter().for_each(|&c| self.f32(c));
    }
    fn string(&mut self, s: &str) {
        self.u32(s.len() as u32);
        self.bytes.extend_from_slice(s.as_bytes());
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
}

impl Reader<'_> {
    fn take<const N: usize>(&mut self) -> Result<[u8; N], FormatError> {
        if self.bytes.len() < N {
            return Err(FormatError::Corrupt("unexpected end of file"));
        }
        let (head, tail) = self.bytes.split_at(N);
        self.bytes = tail;
        Ok(head.try_into().unwrap())
    }
    fn u8(&mut self) -> Result<u8, FormatError> {
        Ok(self.take::<1>()?[0])
    }
    fn u16(&mut self) -> Result<u16, FormatError> {
        Ok(u16::from_le_bytes(self.take()?))
    }
    fn u32(&mut self) -> Result<u32, FormatError> {
        Ok(u32::from_le_bytes(self.take()?))
    }
    fn f32(&mut self) -> Result<f32, FormatError> {
        Ok(f32::from_le_bytes(self.take()?))
    }
    fn vec3(&mut self) -> Result<Vec3, FormatError> {
        Ok(Vec3::new(self.f32()?, self.f32()?, self.f32()?))
    }
    fn string(&mut self) -> Result<String, FormatError> {
        let len = self.u32()? as usize;
        if self.bytes.len() < len {
            return Err(FormatError::Corrupt("string length"));
        }
        let (head, tail) = self.bytes.split_at(len);
        self.bytes = tail;
        String::from_utf8(head.to_vec()).map_err(|_| FormatError::Corrupt("string utf-8"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Quat;

    #[test]
    fn round_trips() {
        let mut module = VoxelModule::new(3);
        for i in 0..40 {
            module.set(UVec3::new(i, i / 2, 63 - i), (i % 3 + 1) as u16);
        }
        let assembly = CanonicalAssembly {
            name: "round trip".into(),
            voxel_size: 1.0 / 32.0,
            palette: vec![
                Material::new([0.0; 3]),
                Material::new([0.1, 0.2, 0.3]),
                Material::new([1.0; 3]),
                Material::new([0.5; 3]),
            ],
            modules: vec![module],
            groups: vec![
                PoseGroupTemplate {
                    name: "base".into(),
                    parent: None,
                    module: 0,
                    rest: Affine3A::IDENTITY,
                    motion: Motion::Static,
                },
                PoseGroupTemplate {
                    name: "arm".into(),
                    parent: Some(0),
                    module: 0,
                    rest: Affine3A::from_rotation_translation(
                        Quat::from_rotation_z(0.4),
                        Vec3::new(1.0, 2.0, 3.0),
                    ),
                    motion: Motion::Hinge {
                        pivot: Vec3::ONE,
                        axis: Vec3::Z,
                        min_angle: -0.2,
                        max_angle: 0.3,
                        response_hz: 0.7,
                    },
                },
            ],
        };
        let bytes = write(&assembly).unwrap();
        assert_eq!(read(&bytes).unwrap(), assembly);
        assert_eq!(
            write(&read(&bytes).unwrap()).unwrap(),
            bytes,
            "encoding is deterministic"
        );
    }

    #[test]
    fn rejects_garbage() {
        assert!(matches!(read(b"nope"), Err(FormatError::BadMagic)));
        let mut bytes = MAGIC.to_vec();
        bytes.extend_from_slice(&99u32.to_le_bytes());
        assert!(matches!(
            read(&bytes),
            Err(FormatError::UnsupportedVersion(99))
        ));
    }
}
