/// A representation of a face unpacked from the bit-packed CPU data
struct UnpackedFace {
    position: vec3<f32>,
    normal_index: u32,
    ao_levels: vec4<f32>,
    texture_id: u32,
};

/// A simplified version of an unpacked face that only includes geometry (cheaper for shadow pass)
struct UnpackedGeometry {
    position: vec3<f32>,
    normal_index: u32,
};

// mask sizes for each category
const POS_MASK: u32 = 31; // 5 bits (0-31)
const AO_MASK: u32 = 3;   // 2 bits (0-3)
const NORM_MASK: u32 = 7; // 3 bits (0-7)
const TEX_MASK: u32 = 63; // 6 bits (0-63)

// bit shift distances
const Y_SHIFT: u32 = 5;
const Z_SHIFT: u32 = 10;
const AO_SHIFT: u32 = 15;
const NORM_SHIFT: u32 = 23;
const TEX_SHIFT: u32 = 26;

/// A function to produce and unpacked face
fn unpack_face(packed: u32) -> UnpackedFace {
    var unpacked_face: UnpackedFace;

    let x = f32(packed & POS_MASK);
    let y = f32((packed >> Y_SHIFT) & POS_MASK);
    let z = f32((packed >> Z_SHIFT) & POS_MASK);
    unpacked_face.position = vec3<f32>(x, y, z);

    let ao0 = f32((packed >> AO_SHIFT) & AO_MASK);
    let ao1 = f32((packed >> (AO_SHIFT + 2)) & AO_MASK);
    let ao2 = f32((packed >> (AO_SHIFT + 4)) & AO_MASK);
    let ao3 = f32((packed >> (AO_SHIFT + 6)) & AO_MASK);
    unpacked_face.ao_levels = vec4<f32>(ao0, ao1, ao2, ao3);

    unpacked_face.normal_index = (packed >> NORM_SHIFT) & NORM_MASK;
    unpacked_face.texture_id = (packed >> TEX_SHIFT) & TEX_MASK;

    return unpacked_face;
}

/// A function to produced an unpacked geometry
fn unpack_face_geometry(packed: u32) -> UnpackedGeometry {
    var out: UnpackedGeometry;

    let x = f32(packed & POS_MASK);
    let y = f32((packed >> Y_SHIFT) & POS_MASK);
    let z = f32((packed >> Z_SHIFT) & POS_MASK);
    out.position = vec3<f32>(x, y, z);

    out.normal_index = (packed >> NORM_SHIFT) & NORM_MASK;

    return out;
}
