use bevy::math::IVec3;
use crate::voxel_world::voxel::Voxel;

pub trait Feature: Send + Sync {
    fn place(&self, origin: IVec3, seed: u32) -> Vec<(IVec3, Voxel)>;
}

pub struct TreeFeature;

impl Feature for TreeFeature {
    fn place(&self, origin: IVec3, seed: u32) -> Vec<(IVec3, Voxel)> {
        let mut changes = Vec::new();
        let h_hash = hash(origin.x, origin.z, seed);
        let height = 4 + (h_hash % 4); // 4 to 7
        
        // Trunk
        for i in 0..height {
            let pos = origin + IVec3::new(0, i as i32, 0);
            changes.push((pos, Voxel::WOOD));
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
                    
                    let leaf_pos = origin + IVec3::new(x, y as i32, z);
                    changes.push((leaf_pos, Voxel::LEAVES));
                }
            }
        }
        changes
    }

}

pub fn hash(x: i32, z: i32, seed: u32) -> u32 {
    let mut h = seed;
    h = h.wrapping_add((x as u32).wrapping_mul(374761393));
    h = h.wrapping_add((z as u32).wrapping_mul(668265263));
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h ^ (h >> 16)
}
