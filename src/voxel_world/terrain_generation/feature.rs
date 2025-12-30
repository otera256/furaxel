use bevy::math::UVec3;
use crate::voxel_world::{terrain_chunk::{TerrainChunkData, TERRAIN_CHUNK_SIZE}, voxel::Voxel};

pub trait Feature: Send + Sync {
    fn place(&self, chunk: &mut TerrainChunkData, local_pos: UVec3, seed: u32);
}

pub struct TreeFeature;

impl Feature for TreeFeature {
    fn place(&self, chunk: &mut TerrainChunkData, local_pos: UVec3, seed: u32) {
        let h_hash = hash(local_pos.x as i32, local_pos.z as i32, seed);
        let height = 4 + (h_hash % 4); // 4 to 7
        
        // Trunk
        for i in 0..height {
            let pos = local_pos + UVec3::new(0, i, 0);
            if pos.y < TERRAIN_CHUNK_SIZE {
                 *chunk.get_local_at_mut(pos) = Voxel::WOOD;
            }
        }
        
        // Leaves
        let leaves_start = height - 2;
        for y in leaves_start..(height + 2) {
            let range = if y >= height { 1 } else { 2 };
            for x in -(range as i32)..=(range as i32) {
                for z in -(range as i32)..=(range as i32) {
                    // Skip corners for rounder look
                    if x.abs() == range as i32 && z.abs() == range as i32 {
                        continue;
                    }
                    
                    let leaf_x = local_pos.x as i32 + x;
                    let leaf_y = local_pos.y as i32 + y as i32;
                    let leaf_z = local_pos.z as i32 + z;

                    if leaf_x >= 0 && leaf_x < TERRAIN_CHUNK_SIZE as i32 &&
                       leaf_y >= 0 && leaf_y < TERRAIN_CHUNK_SIZE as i32 &&
                       leaf_z >= 0 && leaf_z < TERRAIN_CHUNK_SIZE as i32 {
                        let pos = UVec3::new(leaf_x as u32, leaf_y as u32, leaf_z as u32);
                        let voxel = chunk.get_local_at_mut(pos);
                        if *voxel == Voxel::EMPTY {
                            *voxel = Voxel::LEAVES;
                        }
                    }
                }
            }
        }
    }
}

pub fn hash(x: i32, z: i32, seed: u32) -> u32 {
    let mut h = seed;
    h = h.wrapping_add((x as u32).wrapping_mul(374761393));
    h = h.wrapping_add((z as u32).wrapping_mul(668265263));
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h ^ (h >> 16)
}
