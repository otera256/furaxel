use bevy::math::{IVec3, UVec3, Vec3};
use block_mesh::ndshape::ConstShape3u32;

use crate::voxel_world::{
    core::{chunk::Chunk, voxel::Voxel, coordinates::*},
    storage::vertical_rle::{VoxelStorage, RLE_VOXEL_COUNT},
};

type TerrainChunkShape = ConstShape3u32<
    TERRAIN_CHUNK_SIZE,
    TERRAIN_CHUNK_SIZE,
    TERRAIN_CHUNK_SIZE,
>;

pub type PaddedTerrainChunkShape = ConstShape3u32<
    PADDED_TERRAIN_CHUNK_SIZE,
    PADDED_TERRAIN_CHUNK_SIZE,
    PADDED_TERRAIN_CHUNK_SIZE,
>;


#[derive(Debug, Clone)]
pub struct TerrainChunkData {
    storage: VoxelStorage,
    pub position: IVec3,
}

#[allow(dead_code)]
impl TerrainChunkData {
    pub fn chunk_origin(&self) -> IVec3 {
        self.position * IVec3::splat(TERRAIN_CHUNK_SIZE as i32)
    }
    pub fn chunk_origin_f32(&self) -> Vec3 {
        self.chunk_origin().as_vec3() * TERRAIN_CHUNK_LENGTH
    }
    pub fn new_empty(position: IVec3) -> Self {
        Self {
            storage: VoxelStorage::Dense(
                Chunk::new_empty(TerrainChunkShape {}).voxels,
            ),
            position,
        }
    }
    pub fn new_from_fn<F>(position: IVec3, mut f: F) -> Self
    where 
        F: FnMut(IVec3) -> Voxel,
    {
        Self {
            storage: VoxelStorage::Dense(Chunk::new_from_fn(TerrainChunkShape {}, |x, y, z| {
                let world_x = x as i32 + position.x * TERRAIN_CHUNK_SIZE as i32;
                let world_y = y as i32 + position.y * TERRAIN_CHUNK_SIZE as i32;
                let world_z = z as i32 + position.z * TERRAIN_CHUNK_SIZE as i32;
                f(IVec3::new(world_x, world_y, world_z))
            }).voxels),
            position,
        }
    }
    pub fn new_from_fn_local<F>(position: IVec3, mut f: F) -> Self
    where 
        F: FnMut(UVec3) -> Voxel,
    {
        Self {
            storage: VoxelStorage::Dense(Chunk::new_from_fn(TerrainChunkShape {}, |x, y, z| {
                f(UVec3::new(x, y, z))
            }).voxels),
            position,
        }
    }

    #[inline]
    pub fn get_local_at(&self, pos: UVec3) -> Voxel {
        self.storage.get(pos)
    }
    #[inline]
    pub fn set_local_at(&mut self, pos: UVec3, voxel: Voxel) -> bool {
        self.storage.set(pos, voxel)
    }
    #[inline]
    pub fn get_at(&self, world_pos: IVec3) -> Voxel {
        let local_x = (world_pos.x - self.chunk_origin().x) as u32;
        let local_y = (world_pos.y - self.chunk_origin().y) as u32;
        let local_z = (world_pos.z - self.chunk_origin().z) as u32;
        self.get_local_at(UVec3::new(local_x, local_y, local_z))
    }
    #[inline]
    pub fn set_at(&mut self, world_pos: IVec3, voxel: Voxel) -> bool {
        let local_x = (world_pos.x - self.chunk_origin().x) as u32;
        let local_y = (world_pos.y - self.chunk_origin().y) as u32;
        let local_z = (world_pos.z - self.chunk_origin().z) as u32;
        self.set_local_at(UVec3::new(local_x, local_y, local_z), voxel)
    }

    /// Converts a completed generation buffer to the smaller adaptive storage.
    /// Generation code intentionally calls this only after all bulk writes finish.
    pub fn compact(&mut self) {
        if self.storage.is_rle() {
            return;
        }
        let dense = self.storage.to_dense();
        self.storage = VoxelStorage::from_dense(dense)
            .expect("terrain chunk storage must have the fixed chunk size");
    }

    pub fn is_rle(&self) -> bool {
        self.storage.is_rle()
    }

    pub fn memory_bytes(&self) -> usize {
        self.storage.memory_bytes()
    }

    pub fn dense_voxel_count() -> usize {
        RLE_VOXEL_COUNT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completed_terrain_chunk_compacts_without_changing_voxels() {
        let mut chunk = TerrainChunkData::new_from_fn_local(IVec3::new(-2, 1, 3), |position| {
            if position.y < 24 {
                Voxel::STONE
            } else {
                Voxel::EMPTY
            }
        });
        let before = chunk.get_local_at(UVec3::new(7, 23, 11));
        let dense_bytes = TerrainChunkData::dense_voxel_count() * std::mem::size_of::<Voxel>();

        assert!(!chunk.is_rle());
        chunk.compact();

        assert!(chunk.is_rle());
        assert_eq!(chunk.get_local_at(UVec3::new(7, 23, 11)), before);
        assert!(chunk.memory_bytes() < dense_bytes);
        assert!(chunk.set_local_at(UVec3::new(7, 23, 11), Voxel::DIRT));
        assert_eq!(chunk.get_local_at(UVec3::new(7, 23, 11)), Voxel::DIRT);
    }
}
