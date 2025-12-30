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

pub struct CactusFeature;

impl Feature for CactusFeature {
    fn place(&self, origin: IVec3, seed: u32) -> Vec<(IVec3, Voxel)> {
        let mut changes = Vec::new();
        let h_hash = hash(origin.x, origin.z, seed);
        let height = 2 + (h_hash % 3); // 2 to 4

        for i in 0..height {
            let pos = origin + IVec3::new(0, i as i32, 0);
            changes.push((pos, Voxel::CACTUS));
        }
        changes
    }
}

pub struct FlowerFeature;

impl Feature for FlowerFeature {
    fn place(&self, origin: IVec3, seed: u32) -> Vec<(IVec3, Voxel)> {
        let h_hash = hash(origin.x, origin.z, seed);
        let flower_type = if h_hash % 2 == 0 { Voxel::FLOWER_RED } else { Voxel::FLOWER_YELLOW };
        vec![(origin, flower_type)]
    }
}

pub struct PineTreeFeature;

impl Feature for PineTreeFeature {
    fn place(&self, origin: IVec3, seed: u32) -> Vec<(IVec3, Voxel)> {
        let mut changes = Vec::new();
        let h_hash = hash(origin.x, origin.z, seed);
        let height = 6 + (h_hash % 5); // 6 to 10

        // Trunk
        for i in 0..height {
            let pos = origin + IVec3::new(0, i as i32, 0);
            changes.push((pos, Voxel::PINE_LOG));
        }

        // Leaves (Cone shape)
        let leaves_start = 3;
        for y in leaves_start..=height {
            let radius = if y == height { 0 } else if y > height - 3 { 1 } else { 2 };
            
            for x in -(radius as i32)..=(radius as i32) {
                for z in -(radius as i32)..=(radius as i32) {
                    if x == 0 && z == 0 && y < height { continue; } // Don't replace trunk
                    if x.abs() + z.abs() > radius as i32 + 1 { continue; } // Diamond/Cone shape

                    let leaf_pos = origin + IVec3::new(x, y as i32, z);
                    changes.push((leaf_pos, Voxel::PINE_LEAVES));
                }
            }
        }
        // Top leaf
        changes.push((origin + IVec3::new(0, height as i32 + 1, 0), Voxel::PINE_LEAVES));

        changes
    }
}

pub struct BirchTreeFeature;

impl Feature for BirchTreeFeature {
    fn place(&self, origin: IVec3, seed: u32) -> Vec<(IVec3, Voxel)> {
        let mut changes = Vec::new();
        let h_hash = hash(origin.x, origin.z, seed);
        let height = 5 + (h_hash % 3); // 5 to 7
        
        // Trunk
        for i in 0..height {
            let pos = origin + IVec3::new(0, i as i32, 0);
            changes.push((pos, Voxel::BIRCH_LOG));
        }
        
        // Leaves (Similar to normal tree but maybe slightly different)
        let leaves_start = height - 2;
        for y in leaves_start..(height + 2) {
            let range = if y >= height { 1 } else { 2 };
            for x in -(range as i32)..=(range as i32) {
                for z in -(range as i32)..=(range as i32) {
                    if x.abs() == range as i32 && z.abs() == range as i32 {
                        continue;
                    }
                    
                    let leaf_pos = origin + IVec3::new(x, y as i32, z);
                    changes.push((leaf_pos, Voxel::BIRCH_LEAVES));
                }
            }
        }
        changes
    }
}
