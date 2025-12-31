use bevy::math::IVec3;
use crate::voxel_world::voxel::Voxel;

pub trait Feature: Send + Sync {
    fn place(&self, origin: IVec3, seed: u32) -> Vec<(IVec3, Voxel)>;
}

pub struct OakTreeFeature;

impl Feature for OakTreeFeature {
    fn place(&self, origin: IVec3, seed: u32) -> Vec<(IVec3, Voxel)> {
        let mut changes = Vec::new();
        let h_hash = hash(origin.x, origin.z, seed);
        let height = 4 + (h_hash % 4); // 4 to 7
        
        // Trunk
        for i in 0..height {
            let pos = origin + IVec3::new(0, i as i32, 0);
            changes.push((pos, Voxel::OAK_LOG));
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
                    changes.push((leaf_pos, Voxel::OAK_LEAVES));
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

pub struct BigOakTreeFeature;

impl Feature for BigOakTreeFeature {
    fn place(&self, origin: IVec3, seed: u32) -> Vec<(IVec3, Voxel)> {
        let mut logs = Vec::new();
        let mut leaves = Vec::new();
        let h_hash = hash(origin.x, origin.z, seed);
        let height = 10 + (h_hash % 5); // 10 to 14
        
        // Trunk
        for i in 0..height {
            let pos = origin + IVec3::new(0, i as i32, 0);
            logs.push((pos, Voxel::OAK_LOG));
        }

        let mut leaf_centers = Vec::new();
        leaf_centers.push((origin + IVec3::new(0, height as i32, 0), 3)); // Top cluster

        // Branches
        let num_branches = 4 + ((h_hash >> 3) % 3); // 4 to 6 branches
        for i in 0..num_branches {
            let branch_seed = h_hash.wrapping_add(i * 1923);
            let start_h = (height / 3) + (branch_seed % (height / 2));
            let start_pos = origin + IVec3::new(0, start_h as i32, 0);

            // Direction
            let angle = (i as f32 / num_branches as f32) * 6.283 + ((branch_seed % 10) as f32 * 0.1);
            let dir_x = angle.cos();
            let dir_z = angle.sin();
            
            let length = 3 + ((branch_seed >> 4) % 3); // 3 to 5

            for l in 1..=length {
                let offset_x = (dir_x * l as f32).round() as i32;
                let offset_z = (dir_z * l as f32).round() as i32;
                let offset_y = (l as i32 + 1) / 2; // Rise slowly

                let branch_pos = start_pos + IVec3::new(offset_x, offset_y, offset_z);
                logs.push((branch_pos, Voxel::OAK_LOG));
                
                if l == length {
                    leaf_centers.push((branch_pos, 2));
                }
            }
        }

        // Generate Leaves
        for (center, radius) in leaf_centers {
            for x in -radius..=radius {
                for y in -radius..=radius {
                    for z in -radius..=radius {
                        if x*x + y*y + z*z <= radius*radius + 1 {
                            let pos = center + IVec3::new(x, y, z);
                            leaves.push((pos, Voxel::OAK_LEAVES));
                        }
                    }
                }
            }
        }

        // Combine (Leaves first so logs overwrite them if duplicates exist)
        let mut changes = leaves;
        changes.extend(logs);
        changes
    }
}
