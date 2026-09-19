use std::mem::size_of;

use bevy::math::UVec3;

use crate::voxel_world::core::Voxel;

pub const RLE_CHUNK_SIZE: u32 = 64;
pub const RLE_COLUMN_COUNT: usize = (RLE_CHUNK_SIZE * RLE_CHUNK_SIZE) as usize;
pub const RLE_VOXEL_COUNT: usize =
    (RLE_CHUNK_SIZE * RLE_CHUNK_SIZE * RLE_CHUNK_SIZE) as usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerticalRun {
    pub voxel: Voxel,
    pub len: u8,
}

impl VerticalRun {
    fn new(voxel: Voxel, len: u8) -> Self {
        debug_assert!(len > 0);
        Self { voxel, len }
    }
}

/// A fixed-size chunk encoded as contiguous runs along each Y column.
///
/// `column_offsets` has one extra entry, so column `i` owns
/// `runs[offsets[i]..offsets[i + 1]]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerticalRleChunk {
    column_offsets: Box<[u32]>,
    runs: Box<[VerticalRun]>,
}

impl VerticalRleChunk {
    pub fn from_dense(dense: &[Voxel]) -> Result<Self, &'static str> {
        if dense.len() != RLE_VOXEL_COUNT {
            return Err("dense chunk has an unexpected voxel count");
        }

        let mut column_offsets = Vec::with_capacity(RLE_COLUMN_COUNT + 1);
        let mut runs = Vec::new();
        column_offsets.push(0);

        for z in 0..RLE_CHUNK_SIZE {
            for x in 0..RLE_CHUNK_SIZE {
                let mut y = 0;
                while y < RLE_CHUNK_SIZE {
                    let voxel = dense[dense_index(x, y, z)];
                    let mut len = 1;
                    while y + len < RLE_CHUNK_SIZE
                        && dense[dense_index(x, y + len, z)] == voxel
                    {
                        len += 1;
                    }
                    runs.push(VerticalRun::new(voxel, len as u8));
                    y += len;
                }
                column_offsets.push(runs.len() as u32);
            }
        }

        Ok(Self {
            column_offsets: column_offsets.into_boxed_slice(),
            runs: runs.into_boxed_slice(),
        })
    }

    pub fn to_dense(&self) -> Box<[Voxel]> {
        let mut dense = vec![Voxel::EMPTY; RLE_VOXEL_COUNT].into_boxed_slice();
        for z in 0..RLE_CHUNK_SIZE {
            for x in 0..RLE_CHUNK_SIZE {
                let column = column_index(x, z);
                let mut y = 0;
                for run in &self.runs[self.column_range(column)] {
                    for offset in 0..run.len as u32 {
                        dense[dense_index(x, y + offset, z)] = run.voxel;
                    }
                    y += run.len as u32;
                }
                debug_assert_eq!(y, RLE_CHUNK_SIZE);
            }
        }
        dense
    }

    #[inline]
    pub fn get(&self, position: UVec3) -> Voxel {
        debug_assert!(in_bounds(position));
        let column = column_index(position.x, position.z);
        let mut y = 0;
        for run in &self.runs[self.column_range(column)] {
            let next_y = y + run.len as u32;
            if position.y < next_y {
                return run.voxel;
            }
            y = next_y;
        }
        unreachable!("RLE column does not cover the full Y extent");
    }

    /// Updates one voxel and returns whether the value changed.
    pub fn set(&mut self, position: UVec3, voxel: Voxel) -> bool {
        debug_assert!(in_bounds(position));
        let column = column_index(position.x, position.z);
        let start = self.column_offsets[column] as usize;
        let end = self.column_offsets[column + 1] as usize;
        let old_runs = self.runs[start..end].to_vec();

        let mut y = 0;
        let mut replacement = Vec::with_capacity(old_runs.len() + 2);
        let mut changed = false;
        for run in old_runs {
            let run_start = y;
            let run_end = y + run.len as u32;
            if position.y < run_start || position.y >= run_end {
                push_merged(&mut replacement, run);
                y = run_end;
                continue;
            }

            if run.voxel == voxel {
                return false;
            }
            changed = true;
            let before = (position.y - run_start) as u8;
            let after = (run_end - position.y - 1) as u8;
            if before > 0 {
                push_merged(&mut replacement, VerticalRun::new(run.voxel, before));
            }
            push_merged(&mut replacement, VerticalRun::new(voxel, 1));
            if after > 0 {
                push_merged(&mut replacement, VerticalRun::new(run.voxel, after));
            }
            y = run_end;
        }

        debug_assert!(changed);
        let old_len = end - start;
        let new_len = replacement.len();
        let mut runs = Vec::with_capacity(self.runs.len() - old_len + new_len);
        runs.extend_from_slice(&self.runs[..start]);
        runs.extend(replacement);
        runs.extend_from_slice(&self.runs[end..]);
        self.runs = runs.into_boxed_slice();

        let delta = new_len as isize - old_len as isize;
        if delta != 0 {
            for offset in self.column_offsets.iter_mut().skip(column + 1) {
                *offset = (*offset as isize + delta) as u32;
            }
        }
        changed
    }

    #[allow(dead_code)]
    pub fn run_count(&self) -> usize {
        self.runs.len()
    }

    pub fn memory_bytes(&self) -> usize {
        self.column_offsets.len() * size_of::<u32>()
            + self.runs.len() * size_of::<VerticalRun>()
    }

    fn column_range(&self, column: usize) -> std::ops::Range<usize> {
        self.column_offsets[column] as usize..self.column_offsets[column + 1] as usize
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VoxelStorage {
    Dense(Box<[Voxel]>),
    VerticalRle(VerticalRleChunk),
}

#[allow(dead_code)]
impl VoxelStorage {
    pub fn from_dense(dense: Box<[Voxel]>) -> Result<Self, &'static str> {
        let rle = VerticalRleChunk::from_dense(&dense)?;
        let dense_bytes = dense.len() * size_of::<Voxel>();
        if rle.memory_bytes() < dense_bytes {
            Ok(Self::VerticalRle(rle))
        } else {
            Ok(Self::Dense(dense))
        }
    }

    #[inline]
    pub fn get(&self, position: UVec3) -> Voxel {
        match self {
            Self::Dense(dense) => dense[dense_index(position.x, position.y, position.z)],
            Self::VerticalRle(rle) => rle.get(position),
        }
    }

    pub fn set(&mut self, position: UVec3, voxel: Voxel) -> bool {
        let changed = match self {
            Self::Dense(dense) => {
                let target = &mut dense[dense_index(position.x, position.y, position.z)];
                if *target == voxel {
                    return false;
                }
                *target = voxel;
                true
            }
            Self::VerticalRle(rle) => rle.set(position, voxel),
        };

        if changed
            && matches!(self, Self::VerticalRle(rle) if rle.memory_bytes() >= RLE_VOXEL_COUNT * size_of::<Voxel>())
        {
            let dense = self.to_dense();
            *self = Self::Dense(dense);
        }
        changed
    }

    pub fn to_dense(&self) -> Box<[Voxel]> {
        match self {
            Self::Dense(dense) => dense.clone(),
            Self::VerticalRle(rle) => rle.to_dense(),
        }
    }

    pub fn memory_bytes(&self) -> usize {
        match self {
            Self::Dense(dense) => dense.len() * size_of::<Voxel>(),
            Self::VerticalRle(rle) => rle.memory_bytes(),
        }
    }

    pub fn is_rle(&self) -> bool {
        matches!(self, Self::VerticalRle(_))
    }
}

fn push_merged(runs: &mut Vec<VerticalRun>, run: VerticalRun) {
    if let Some(previous) = runs.last_mut()
        && previous.voxel == run.voxel
    {
        previous.len += run.len;
    } else {
        runs.push(run);
    }
}

#[inline]
fn in_bounds(position: UVec3) -> bool {
    position.x < RLE_CHUNK_SIZE
        && position.y < RLE_CHUNK_SIZE
        && position.z < RLE_CHUNK_SIZE
}

#[inline]
fn column_index(x: u32, z: u32) -> usize {
    (z * RLE_CHUNK_SIZE + x) as usize
}

#[inline]
fn dense_index(x: u32, y: u32, z: u32) -> usize {
    (x + RLE_CHUNK_SIZE * (y + RLE_CHUNK_SIZE * z)) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dense_from_fn(mut f: impl FnMut(u32, u32, u32) -> Voxel) -> Box<[Voxel]> {
        let mut dense = Vec::with_capacity(RLE_VOXEL_COUNT);
        for z in 0..RLE_CHUNK_SIZE {
            for y in 0..RLE_CHUNK_SIZE {
                for x in 0..RLE_CHUNK_SIZE {
                    dense.push(f(x, y, z));
                }
            }
        }
        dense.into_boxed_slice()
    }

    #[test]
    fn uniform_chunk_compresses_and_round_trips() {
        let dense = dense_from_fn(|_, _, _| Voxel::STONE);
        let rle = VerticalRleChunk::from_dense(&dense).unwrap();

        assert_eq!(rle.run_count(), RLE_COLUMN_COUNT);
        assert!(rle.memory_bytes() < dense.len() * size_of::<Voxel>());
        assert_eq!(rle.to_dense(), dense);
    }

    #[test]
    fn editing_splits_and_merges_vertical_runs() {
        let dense = dense_from_fn(|_, y, _| if y < 32 { Voxel::STONE } else { Voxel::DIRT });
        let mut rle = VerticalRleChunk::from_dense(&dense).unwrap();
        let position = UVec3::new(3, 31, 7);

        assert!(rle.set(position, Voxel::EMPTY));
        assert_eq!(rle.get(position), Voxel::EMPTY);
        assert!(rle.set(position, Voxel::STONE));
        assert_eq!(rle.to_dense(), dense);
    }

    #[test]
    fn incompressible_data_keeps_dense_representation() {
        let dense = dense_from_fn(|x, y, z| Voxel::new(((x + y + z) % 2 + 2) as u16));
        let storage = VoxelStorage::from_dense(dense).unwrap();

        assert!(!storage.is_rle());
    }

    #[test]
    fn adaptive_storage_matches_dense_after_random_edits() {
        let dense = dense_from_fn(|_, y, _| if y < 20 { Voxel::STONE } else { Voxel::EMPTY });
        let expected = dense.clone();
        let mut storage = VoxelStorage::from_dense(dense).unwrap();
        let edits = [
            (UVec3::new(0, 0, 0), Voxel::DIRT),
            (UVec3::new(63, 63, 63), Voxel::STONE),
            (UVec3::new(12, 20, 8), Voxel::WATER),
        ];
        let mut expected = expected;
        for (position, voxel) in edits {
            assert!(storage.set(position, voxel));
            expected[dense_index(position.x, position.y, position.z)] = voxel;
        }

        assert_eq!(storage.to_dense(), expected);
    }

    #[test]
    fn repeated_edits_match_a_dense_reference() {
        let dense = dense_from_fn(|_, y, _| if y < 24 { Voxel::STONE } else { Voxel::EMPTY });
        let mut expected = dense.clone();
        let mut storage = VoxelStorage::from_dense(dense).unwrap();

        for index in 0..1024u32 {
            let position = UVec3::new(
                (index.wrapping_mul(37)) % RLE_CHUNK_SIZE,
                (index.wrapping_mul(53)) % RLE_CHUNK_SIZE,
                (index.wrapping_mul(91)) % RLE_CHUNK_SIZE,
            );
            let voxel = match index % 4 {
                0 => Voxel::EMPTY,
                1 => Voxel::DIRT,
                2 => Voxel::STONE,
                _ => Voxel::WATER,
            };
            let dense_index = dense_index(position.x, position.y, position.z);
            let expected_changed = expected[dense_index] != voxel;
            assert_eq!(storage.set(position, voxel), expected_changed);
            expected[dense_index] = voxel;
            assert_eq!(storage.get(position), voxel);
        }

        assert_eq!(storage.to_dense(), expected);
    }

    #[test]
    fn invalid_dense_size_is_rejected() {
        assert!(VerticalRleChunk::from_dense(&[]).is_err());
        assert!(VoxelStorage::from_dense(Vec::new().into_boxed_slice()).is_err());
    }
}
