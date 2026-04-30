use bevy::prelude::{Component, Deref, DerefMut};
use crate::prelude::*;
use crate::world::chunk::CHUNK_SIDE_LENGTH;
use std::fmt::{Display, Formatter};
use std::mem::MaybeUninit;
use std::sync::Arc;

/// A type-safe wrapper for a Level of Detail value.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deref, DerefMut, Default)]
pub struct ChunkLod(pub u8);

impl Display for ChunkLod {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ChunkLod {
    /// Returns the side length (e.g., 32, 16, 8) for this LOD.
    #[inline(always)]
    pub fn sidelength(self) -> usize {
        CHUNK_SIDE_LENGTH >> self.0
    }

    /// Returns the area (sidelength^2) for this LOD.
    #[inline(always)]
    pub fn sidelength_pow2(self) -> usize {
        let size = self.sidelength();
        size * size
    }

    /// Returns the volume (sidelength^3) for this LOD.
    #[inline(always)]
    pub fn sidelength_pow3(self) -> usize {
        let size = self.sidelength();
        size * size * size
    }

    /// Calculates the bit-shift required for Z indexing at this LOD.
    #[inline(always)]
    pub fn z_shift(self) -> u8 {
        self.sidelength().trailing_zeros() as u8
    }

    /// Calculates the bit-shift required for Y indexing at this LOD.
    #[inline(always)]
    pub fn y_shift(self) -> u8 {
        self.z_shift() * 2
    }
}

// INFO: -----------------------------------
//         3D chunk volume accessors
// -----------------------------------------

/// A temporary read-only view into a volume's data optimized for hot loops.
#[derive(Clone, Copy)]
pub struct VolumeDataView<'a, T> {
    data: &'a [T],
    x_shift: u8,
    z_shift: u8,
}

impl<'a, T: Copy> VolumeDataView<'a, T> {
    /// Gets a piece of data from the chunk volume.
    ///
    /// Caller must ensure x, y, z are within bounds or undefined behavior will occur.
    #[inline(always)]
    pub fn get_data(&self, x: usize, y: usize, z: usize) -> T {
        let index = (x << self.x_shift) | (z << self.z_shift) | y;

        if cfg!(debug_assertions) {
            let size = 1 << self.z_shift;
            if x >= size || y >= size || z >= size {
                error!(
                    "get_data: Out of bounds: ({}, {}, {}) in chunk size {}",
                    x, y, z, size
                );
            }
        }

        unsafe { *self.data.get_unchecked(index) }
    }
}

/// A temporary accessor for safe, high-speed batch writes to a chunk volume.
pub struct VolumeDataWriter<'a, T> {
    data: &'a mut [T],
    x_shift: u8,
    z_shift: u8,
}

impl<'a, T: Copy> VolumeDataWriter<'a, T> {
    /// Sets data in the chunk volume to a given value.
    ///
    /// Caller must ensure x, y, z are within bounds or undefined behavior will occur.
    #[inline(always)]
    pub fn set_data(&mut self, x: usize, y: usize, z: usize, data: T) {
        let index = (x << self.x_shift) | (z << self.z_shift) | y;

        if cfg!(debug_assertions) {
            let size = 1 << self.z_shift;
            if x >= size || y >= size || z >= size {
                panic!(
                    "VolumeDataWriter::set_block: Out of bounds ({}, {}, {}) for size {}",
                    x, y, z, size
                );
            }
            // also check index just in case logic is wrong
            if index >= self.data.len() {
                panic!(
                    "VolumeDataWriter::set_block: Calculated index {} is out of bounds {}",
                    index,
                    self.data.len()
                );
            }
        }

        unsafe {
            *self.data.get_unchecked_mut(index) = data;
        }
    }

    /// Sets a piece of data from the chunk volume by index.
    ///
    /// Caller must insure the index is within bounds or undefined behavior will occur.
    pub fn set_at_index(&mut self, index: usize, data: T) {
        unsafe { *self.data.get_unchecked_mut(index) = data };
    }

    /// Gets a piece of data from the chunk volume by coordinate.
    ///
    /// Caller must ensure x, y, z are within bounds or undefined behavior will occur.
    #[inline(always)]
    pub fn get_data(&self, x: usize, y: usize, z: usize) -> T {
        let index = (x << self.x_shift) | (z << self.z_shift) | y;

        if cfg!(debug_assertions) {
            let size = 1 << self.z_shift;
            if x >= size || y >= size || z >= size {
                error!(
                    "get_data: Out of bounds: ({}, {}, {}) in chunk size {}",
                    x, y, z, size
                );
            }
        }

        unsafe { *self.data.get_unchecked(index) }
    }

    /// Gets a piece of data from the chunk volume by index.
    ///
    /// Caller must insure the index is within bounds or undefined behavior will occur.
    #[inline(always)]
    pub fn get_at_index(&self, index: usize) -> T {
        unsafe { *self.data.get_unchecked(index) }
    }

    /// Bulk sets the entire volume to a value (memset).
    ///
    /// MUCH faster than setting data one by one in a loop.
    #[inline(always)]
    pub fn fill(&mut self, value: T)
    where
        T: Copy,
    {
        self.data.fill(value);
    }

    /// Fills a slice of the volume efficiently with a single value.
    ///
    /// Faster than setting the slice elements individually.
    #[inline(always)]
    pub fn fill_slice(&mut self, start: usize, end: usize, value: T)
    where
        T: Copy,
    {
        unsafe {
            let slice = self.data.get_unchecked_mut(start..end);
            slice.fill(value);
        }
    }

    /// Copies data from a slice directly into this volume (memcpy).
    ///
    /// The source slice MUST be the same size as the chunk volume.
    pub fn copy_from_slice(&mut self, source: &[T])
    where
        T: Copy,
    {
        self.data.copy_from_slice(source);
    }
}

// INFO: -----------------------------------
//         3d chunk volume container
// -----------------------------------------

/// Generic, LOD-aware, 3D container for chunk voxel data.
#[derive(Clone)]
pub struct ChunkVolumeData<T: Send + Sync + 'static> {
    data: Arc<[T]>,

    /// The size of one edge (e.g., 32, 16, 8, ...).
    size: usize,
    /// The level of detail (0 = full detail, 1 = half size, etc.).
    lod: ChunkLod,
    /// Pre-calculated shift for X (e.g., log2(size) * 2).
    pub x_shift: u8,
    /// Pre-calculated shift for Z (e.g., log2(size)).
    pub z_shift: u8,
}

impl<T: Copy + Send + Sync + 'static> ChunkVolumeData<T> {
    /// Creates a new volume with all bits set to zero.
    pub fn new_zeroed(lod: ChunkLod) -> Self {
        let size = CHUNK_SIDE_LENGTH >> *lod;
        let num_elements = size.pow(3);

        // allocate uninitialized mem
        let mut uninit_arc: Arc<[MaybeUninit<T>]> = Arc::new_uninit_slice(num_elements);
        // get a raw mutable pointer to the data
        let ptr = Arc::get_mut(&mut uninit_arc).unwrap().as_mut_ptr();
        // memset to 0
        unsafe {
            std::ptr::write_bytes(ptr, 0, num_elements);
        }
        // freeze into an initialized Arc
        let data: Arc<[T]> = unsafe { uninit_arc.assume_init() };

        // calculate shifts
        let z_shift = size.trailing_zeros() as u8;
        let x_shift = z_shift * 2;

        Self {
            data,
            size,
            lod,
            x_shift,
            z_shift,
        }
    }

    /// Creates a new volume component filled with `value`.
    pub fn new_filled(lod: ChunkLod, value: T) -> Self {
        let size = CHUNK_SIDE_LENGTH >> *lod;
        let num_elements = size.pow(3);

        // allocate uninitialized mem
        let mut uninit_arc: Arc<[MaybeUninit<T>]> = Arc::new_uninit_slice(num_elements);
        // get a raw mutable slice
        let uninit_slice = Arc::get_mut(&mut uninit_arc).unwrap();
        // fill memory
        for element in uninit_slice.iter_mut() {
            element.write(value);
        }
        // freeze into an initialized Arc
        let data: Arc<[T]> = unsafe { uninit_arc.assume_init() };

        // calculate shifts
        let z_shift = size.trailing_zeros() as u8;
        let x_shift = z_shift * 2;

        Self {
            data,
            size,
            lod,
            x_shift,
            z_shift,
        }
    }

    /// Creates a new volume component from a vec `data` for a specific `lod`.
    ///
    /// Panics if the length of `data` does not match the expected size.
    pub fn from_vec(lod: ChunkLod, data: Vec<T>) -> Self {
        let size = CHUNK_SIDE_LENGTH >> *lod;
        let expected_len = size.pow(3);

        if data.len() != expected_len {
            panic!(
                "ChunkVolumeData::new: Data length ({}) does not match expected length ({}) for LOD {} (size {}).",
                data.len(),
                expected_len,
                lod,
                size
            );
        }

        let z_shift = size.trailing_zeros() as u8;
        let x_shift = z_shift * 2;

        Self {
            data: data.into_boxed_slice().into(),
            size,
            lod,
            x_shift,
            z_shift,
        }
    }

    /// Returns the size of one edge of the chunk volume.
    pub fn size(&self) -> usize {
        self.size
    }

    /// Returns the level of detail of the chunk volume.
    pub fn lod(&self) -> ChunkLod {
        self.lod
    }

    /// Returns a mutable reference to the underlying Vec, cloning if it's shared.
    #[inline(always)]
    pub fn get_data_mut(&mut self) -> &mut [T] {
        Arc::make_mut(&mut self.data)
    }

    /// Returns an immutable view of the underlying volume data.
    #[inline(always)]
    pub fn get_data_view(&self) -> VolumeDataView<'_, T> {
        VolumeDataView {
            data: &self.data,
            x_shift: self.x_shift,
            z_shift: self.z_shift,
        }
    }

    /// Returns a mutable accessor to the underlying volume data.
    #[inline(always)]
    pub fn get_data_writer(&mut self) -> VolumeDataWriter<'_, T> {
        VolumeDataWriter {
            data: Arc::make_mut(&mut self.data),
            x_shift: self.x_shift,
            z_shift: self.z_shift,
        }
    }

    /// Gets the data at the given local coordinates.
    ///
    /// Has undefined behavior if called on indices out of chunk bounds.
    #[inline(always)]
    pub fn get_data_unchecked(&self, x: usize, y: usize, z: usize) -> T {
        let index = (x << self.x_shift) | (z << self.z_shift) | y;

        if cfg!(debug_assertions) && (x >= self.size || y >= self.size || z >= self.size) {
            error!(
                "get_data_unchecked: Attempted to access voxel data out of bounds: ({}, {}, {}) in a chunk of size {}",
                x, y, z, self.size
            );
        }

        unsafe { *self.data.get_unchecked(index) }
    }
}

// INFO: -----------------------------
//         2D column accessors
// -----------------------------------

/// A temporary read-only view into a column's data optimized for hot loops.
#[derive(Clone, Copy)]
pub struct ColumnDataView<'a, T> {
    data: &'a [T],
    x_shift: u8, // Shift for X (outer)
    size: usize, // Needed for debug assertions
}

impl<'a, T: Copy> ColumnDataView<'a, T> {
    /// Gets a piece of data from the chunk column.
    #[inline(always)]
    pub fn get_data(&self, x: usize, z: usize) -> T {
        let index = (x << self.x_shift) | z;
        if cfg!(debug_assertions) && (x >= self.size || z >= self.size) {
            error!(
                "get_data: Out of bounds: ({}, {}) in chunk 2D size {}",
                x, z, self.size
            );
        }
        unsafe { *self.data.get_unchecked(index) }
    }
}

/// A temporary accessor for safe, high-speed batch writes to a chunk column.
pub struct ColumnDataWriter<'a, T> {
    data: &'a mut [T],
    x_shift: u8,
    size: usize,
}

impl<'a, T: Copy> ColumnDataWriter<'a, T> {
    /// Sets data in the chunk column to a given value.
    #[inline(always)]
    pub fn set_data(&mut self, x: usize, z: usize, data: T) {
        let index = (x << self.x_shift) | z;
        if cfg!(debug_assertions) && (x >= self.size || z >= self.size) {
            panic!(
                "ColumnDataWriter::set_block: Out of bounds ({}, {}) for size {}",
                x, z, self.size
            );
        }
        unsafe {
            *self.data.get_unchecked_mut(index) = data;
        }
    }

    /// Sets a piece of data from the chunk volume by index.
    ///
    /// Caller must insure the index is within bounds or undefined behavior will occur.
    pub fn set_at_index(&mut self, index: usize, data: T) {
        if cfg!(debug_assertions) && (index >= self.size * self.size) {
            panic!(
                "ColumnDataWriter::set_at_index: Out of bounds index ({}) for size {}",
                index, self.size
            );
        }

        unsafe { *self.data.get_unchecked_mut(index) = data };
    }

    /// Fills the entire column volume with a value.
    pub fn fill(&mut self, value: T) {
        self.data.fill(value);
    }
}

// INFO: -----------------------------------
//         2D chunk column container
// -----------------------------------------

/// Generic, LOD-aware, 2D container for chunk column data.
#[derive(Clone)]
pub struct ChunkColumnData<T: Send + Sync + 'static> {
    data: Arc<[T]>,

    /// The size of one edge (e.g., 32, 16, 8, ...).
    size: usize,

    /// The level of detail (0 = full detail, 1 = half size, etc.).
    lod: ChunkLod,

    /// Pre-calculated shift for X (e.g., log2(size)).
    x_shift: u8,
}

impl<T: Copy + Send + Sync + 'static> ChunkColumnData<T> {
    /// Creates a new column with all bits set to zero.
    pub fn new_zeroed(lod: ChunkLod) -> Self {
        let size = CHUNK_SIDE_LENGTH >> *lod;
        let num_elements = size.pow(2);

        let mut uninit_arc: Arc<[MaybeUninit<T>]> = Arc::new_uninit_slice(num_elements);
        let ptr = Arc::get_mut(&mut uninit_arc).unwrap().as_mut_ptr();
        unsafe {
            std::ptr::write_bytes(ptr, 0, num_elements);
        }
        let data: Arc<[T]> = unsafe { uninit_arc.assume_init() };

        Self {
            data,
            size,
            lod,
            x_shift: size.trailing_zeros() as u8,
        }
    }

    /// Creates a new column component filled with `value`.
    pub fn new_filled(lod: ChunkLod, value: T) -> Self {
        let size = CHUNK_SIDE_LENGTH >> *lod;
        let num_elements = size.pow(2);

        let mut uninit_arc: Arc<[MaybeUninit<T>]> = Arc::new_uninit_slice(num_elements);
        let uninit_slice = Arc::get_mut(&mut uninit_arc).unwrap();
        for element in uninit_slice.iter_mut() {
            element.write(value);
        }
        let data: Arc<[T]> = unsafe { uninit_arc.assume_init() };

        Self {
            data,
            size,
            lod,
            x_shift: size.trailing_zeros() as u8,
        }
    }

    /// Creates a new column component from a vec `data`.
    pub fn from_vec(lod: ChunkLod, data: Vec<T>) -> Self {
        let size = CHUNK_SIDE_LENGTH >> *lod;
        let expected_len = size.pow(2);

        if data.len() != expected_len {
            panic!(/* ... error message ... */);
        }

        Self {
            data: data.into_boxed_slice().into(),
            size,
            lod,
            x_shift: size.trailing_zeros() as u8,
        }
    }

    /// Returns the size of one edge of the chunk column.
    pub fn size(&self) -> usize {
        self.size
    }

    /// Returns the level of detail of the chunk column.
    pub fn lod(&self) -> ChunkLod {
        self.lod
    }

    /// Returns an immutable view of the underlying column data.
    #[inline(always)]
    pub fn get_data_view(&self) -> ColumnDataView<'_, T> {
        ColumnDataView {
            data: &self.data,
            x_shift: self.x_shift,
            size: self.size,
        }
    }

    /// Returns a mutable accessor to the underlying column data.
    #[inline(always)]
    pub fn get_data_writer(&mut self) -> ColumnDataWriter<'_, T> {
        ColumnDataWriter {
            data: Arc::make_mut(&mut self.data),
            x_shift: self.x_shift,
            size: self.size,
        }
    }

    /// Gets the data at the given local coordinates.
    #[inline(always)]
    pub fn get_data_unchecked(&self, x: usize, z: usize) -> T {
        if cfg!(debug_assertions) && (x >= self.size || z >= self.size) {
            error!(
                "get_data_unchecked: Attempted to access voxel data out of bounds: ({}, {}) in 2D chunk of size {}",
                x, z, self.size
            );
        }
        let index = (x << self.x_shift) | z;
        unsafe { *self.data.get_unchecked(index) }
    }
}
