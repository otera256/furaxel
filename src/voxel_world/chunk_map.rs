use std::vec;
use std::collections::HashSet;

use bevy::{ecs::resource::Resource, math::{IVec3, UVec3}, platform::collections::HashMap};
use block_mesh::ndshape::ConstShape;
use itertools::iproduct;

use crate::voxel_world::{chunk::Chunk, terrain_chunk::{PaddedTerrainChunkShape, TERRAIN_CHUNK_SIZE, TerrainChunkData}, voxel::Voxel};

#[derive(Debug, Resource, Default)]
pub struct ChunkMap {
    pub chunks: HashMap<IVec3, TerrainChunkData>,
}

#[allow(dead_code)]
impl ChunkMap {
    pub fn new() -> Self {
        Self {
            chunks: HashMap::new(),
        }
    }

    pub fn insert(&mut self, chunk: TerrainChunkData) {
        self.chunks.insert(chunk.position, chunk);
    }

    pub fn get(&self, position: &IVec3) -> Option<&TerrainChunkData> {
        self.chunks.get(position)
    }
    pub fn get_slice(&self, position: &IVec3) -> Option<&[Voxel]> {
        let chunk = self.chunks.get(position)?;
        Some(chunk.chunk.as_slice())
    }
    // meshingする際に使用。隣接する6チャンクの1層分を取り込んで取得する
    // positionのチャンクが存在しないときはNoneを返す
    // 隣接するチャンクが存在しないときはEMPTY_VOXELで埋める
    pub fn get_padded_chunk_vec(&self, position: &IVec3) -> Option<Chunk<PaddedTerrainChunkShape>> {
        let center_chunk = self.chunks.get(position)?;
        let mut padded_voxels = vec![Voxel::EMPTY; PaddedTerrainChunkShape::USIZE];
        // 中心チャンクをコピー
        for (x, y, z) in iproduct!(0..TERRAIN_CHUNK_SIZE, 0..TERRAIN_CHUNK_SIZE, 0..TERRAIN_CHUNK_SIZE) {
            let voxel = center_chunk.get_local_at(UVec3::new(x, y, z));
            let index = PaddedTerrainChunkShape::linearize([x + 1, y + 1, z + 1]) as usize;
            padded_voxels[index] = voxel;
        }
        // 6方向の隣接チャンクをコピー
        // -X (West) 方向
        if let Some(west_chunk) = self.chunks.get(&( *position + IVec3::new(-1, 0, 0))) {
            for (y, z) in iproduct!(0..TERRAIN_CHUNK_SIZE, 0..TERRAIN_CHUNK_SIZE) {
                let voxel = west_chunk.get_local_at(UVec3::new(TERRAIN_CHUNK_SIZE - 1, y, z));
                let index = PaddedTerrainChunkShape::linearize([0, y + 1, z + 1]) as usize;
                padded_voxels[index] = voxel;
            }
        }
        // +X (East) 方向
        if let Some(east_chunk) = self.chunks.get(&( *position + IVec3::new(1, 0, 0))) {
            for (y, z) in iproduct!(0..TERRAIN_CHUNK_SIZE, 0..TERRAIN_CHUNK_SIZE) {
                let voxel = east_chunk.get_local_at(UVec3::new(0, y, z));
                let index = PaddedTerrainChunkShape::linearize([TERRAIN_CHUNK_SIZE + 1, y + 1, z + 1]) as usize;
                padded_voxels[index] = voxel;
            }
        }
        // -Y (Down) 方向
        if let Some(down_chunk) = self.chunks.get(&( *position + IVec3::new(0, -1, 0))) {
            for (x, z) in iproduct!(0..TERRAIN_CHUNK_SIZE, 0..TERRAIN_CHUNK_SIZE) {
                let voxel = down_chunk.get_local_at(UVec3::new(x, TERRAIN_CHUNK_SIZE - 1, z));
                let index = PaddedTerrainChunkShape::linearize([x + 1, 0, z + 1]) as usize;
                padded_voxels[index] = voxel;
            }
        }
        // +Y (Up) 方向
        if let Some(up_chunk) = self.chunks.get(&( *position + IVec3::new(0, 1, 0))) {
            for (x, z) in iproduct!(0..TERRAIN_CHUNK_SIZE, 0..TERRAIN_CHUNK_SIZE) {
                let voxel = up_chunk.get_local_at(UVec3::new(x, 0, z));
                let index = PaddedTerrainChunkShape::linearize([x + 1, TERRAIN_CHUNK_SIZE + 1, z + 1]) as usize;
                padded_voxels[index] = voxel;
            }
        }
        // -Z (North) 方向
        if let Some(north_chunk) = self.chunks.get(&( *position + IVec3::new(0, 0, -1))) {
            for (x, y) in iproduct!(0..TERRAIN_CHUNK_SIZE, 0..TERRAIN_CHUNK_SIZE) {
                let voxel = north_chunk.get_local_at(UVec3::new(x, y, TERRAIN_CHUNK_SIZE - 1));
                let index = PaddedTerrainChunkShape::linearize([x + 1, y + 1, 0]) as usize;
                padded_voxels[index] = voxel;
            }
        }
        // +Z (South) 方向
        if let Some(south_chunk) = self.chunks.get(&( *position + IVec3::new(0, 0, 1))) {
            for (x, y) in iproduct!(0..TERRAIN_CHUNK_SIZE, 0..TERRAIN_CHUNK_SIZE) {
                let voxel = south_chunk.get_local_at(UVec3::new(x, y, 0));
                let index = PaddedTerrainChunkShape::linearize([x + 1, y + 1, TERRAIN_CHUNK_SIZE + 1]) as usize;
                padded_voxels[index] = voxel;
            }
        }
        Some(Chunk {
            voxels: padded_voxels.into_boxed_slice(),
            shape: PaddedTerrainChunkShape {},
        })
    }

    pub fn get_mut(&mut self, position: &IVec3) -> Option<&mut TerrainChunkData> {
        self.chunks.get_mut(position)
    }

    pub fn set_bulk(&mut self, changes: Vec<(IVec3, Voxel)>) -> HashSet<IVec3> {
        let mut chunks_to_update = HashSet::new();
        let mut changes_by_chunk: HashMap<IVec3, Vec<(IVec3, Voxel)>> = HashMap::new();
        
        for (world_pos, voxel) in changes {
            let chunk_pos = world_pos.div_euclid(IVec3::splat(TERRAIN_CHUNK_SIZE as i32));
            changes_by_chunk.entry(chunk_pos).or_default().push((world_pos, voxel));
        }

        for (chunk_pos, chunk_changes) in changes_by_chunk {
            if let Some(chunk) = self.chunks.get_mut(&chunk_pos) {
                let mut modified = false;
                for (world_pos, voxel) in chunk_changes {
                    let local_pos = (world_pos.rem_euclid(IVec3::splat(TERRAIN_CHUNK_SIZE as i32))).as_uvec3();
                    let target_voxel = chunk.get_local_at_mut(local_pos);
                    
                    // Only overwrite if the target is empty, water, or snow (soft blocks)
                    if target_voxel.id == Voxel::EMPTY.id || target_voxel.id == Voxel::WATER.id || target_voxel.id == Voxel::SNOW.id {
                        *target_voxel = voxel;
                        modified = true;
                    }
                }
                if modified {
                    chunks_to_update.insert(chunk_pos);
                }
            }
        }
        chunks_to_update
    }

    pub fn get_at(&self, world_pos: IVec3) -> Option<Voxel> {
        let chunk_pos = world_pos.div_euclid(IVec3::splat(TERRAIN_CHUNK_SIZE as i32));
        let chunk = self.chunks.get(&chunk_pos)?;
        Some(chunk.get_at(world_pos))
    }
    pub fn get_at_mut(&mut self, world_pos: IVec3) -> Option<&mut Voxel> {
        let chunk_pos = world_pos.div_euclid(IVec3::splat(TERRAIN_CHUNK_SIZE as i32));
        let chunk = self.chunks.get_mut(&chunk_pos)?;
        Some(chunk.get_at_mut(world_pos))
    }
}