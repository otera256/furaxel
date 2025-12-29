use bevy::math::{IVec3, UVec3, Vec3};
use block_mesh::ndshape::ConstShape3u32;

use crate::voxel_world::{chunk::Chunk, voxel::{VOXEL_SIZE, Voxel}};

pub const TERRAIN_CHUNK_SIZE: u32 = 32;
pub const PADDED_TERRAIN_CHUNK_SIZE: u32 = TERRAIN_CHUNK_SIZE + 2;
pub const TERRAIN_CHUNK_LENGTH: f32 = TERRAIN_CHUNK_SIZE as f32 * VOXEL_SIZE;

type TerrainChunkShape = ConstShape3u32<
    TERRAIN_CHUNK_SIZE,
    TERRAIN_CHUNK_SIZE,
    TERRAIN_CHUNK_SIZE,
>;

pub(super) type PaddedTerrainChunkShape = ConstShape3u32<
    PADDED_TERRAIN_CHUNK_SIZE,
    PADDED_TERRAIN_CHUNK_SIZE,
    PADDED_TERRAIN_CHUNK_SIZE,
>;


#[derive(Debug, Clone)]
pub struct TerrainChunkData {
    pub chunk: Chunk<TerrainChunkShape>,
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
            chunk: Chunk::new_empty(TerrainChunkShape {}),
            position,
        }
    }
    pub fn new_from_fn<F>(position: IVec3, mut f: F) -> Self
    where 
        F: FnMut(IVec3) -> Voxel,
    {
        Self {
            chunk: Chunk::new_from_fn(TerrainChunkShape {}, |x, y, z| {
                let world_x = x as i32 + position.x * TERRAIN_CHUNK_SIZE as i32;
                let world_y = y as i32 + position.y * TERRAIN_CHUNK_SIZE as i32;
                let world_z = z as i32 + position.z * TERRAIN_CHUNK_SIZE as i32;
                f(IVec3::new(world_x, world_y, world_z))
            }),
            position,
        }
    }

    #[inline]
    pub fn get_local_at(&self, pos: UVec3) -> Voxel {
        self.chunk.get_at(pos)
    }
    #[inline]
    pub fn get_local_at_mut(&mut self, pos: UVec3) -> &mut Voxel {
        self.chunk.get_at_mut(pos)
    }
    #[inline]
    pub fn get_at(&self, world_pos: IVec3) -> Voxel {
        let local_x = (world_pos.x - self.chunk_origin().x) as u32;
        let local_y = (world_pos.y - self.chunk_origin().y) as u32;
        let local_z = (world_pos.z - self.chunk_origin().z) as u32;
        self.get_local_at(UVec3::new(local_x, local_y, local_z))
    }
    #[inline]
    pub fn get_at_mut(&mut self, world_pos: IVec3) -> &mut Voxel {
        let local_x = (world_pos.x - self.chunk_origin().x) as u32;
        let local_y = (world_pos.y - self.chunk_origin().y) as u32;
        let local_z = (world_pos.z - self.chunk_origin().z) as u32;
        self.get_local_at_mut(UVec3::new(local_x, local_y, local_z))
    }
}