use bevy::prelude::*;
use block_mesh::ndshape::{AbstractShape, ConstShape2u32};
use noise::{Billow, Fbm, MultiFractal, NoiseFn, Perlin, RidgedMulti, ScaleBias, Select};
use crate::voxel_world::{
    terrain_chunk::{TERRAIN_CHUNK_SIZE, TerrainChunkData},
    voxel::{VOXEL_SIZE, Voxel},
};
use super::{biomes::BiomeRegistry, feature};

type AltitudeMapShape = ConstShape2u32<TERRAIN_CHUNK_SIZE, TERRAIN_CHUNK_SIZE>;

pub fn generate_altitude_map(
    seed: u32,
    chunk_xz: IVec2,
    config: &BiomeRegistry,
) -> (Vec<i32>, Vec<u8>) {
    // --- Noise Generation Logic ---
    let base_continent = Fbm::<Perlin>::new(seed)
        .set_frequency(0.001)
        .set_persistence(0.5)
        .set_lacunarity(2.2)
        .set_octaves(6);
    let mountains = RidgedMulti::<Perlin>::new(seed + 1)
        .set_frequency(0.003)
        .set_octaves(10);
    let plains_base = Billow::<Perlin>::new(seed + 2)
        .set_frequency(0.002)
        .set_octaves(8);
    let plains = ScaleBias::new(plains_base).set_scale(0.125);
    let terrain_generator = Select::new(plains, mountains, base_continent)
        .set_bounds(0.2, 100.0)
        .set_falloff(0.3);

    let mut altitude_map = vec![0i32; (TERRAIN_CHUNK_SIZE * TERRAIN_CHUNK_SIZE) as usize];
    let mut biome_map = vec![0u8; (TERRAIN_CHUNK_SIZE * TERRAIN_CHUNK_SIZE) as usize];

    for z in 0..TERRAIN_CHUNK_SIZE {
        for x in 0..TERRAIN_CHUNK_SIZE {
            let world_xz = (IVec2::new(x as i32, z as i32)
                + chunk_xz * IVec2::splat(TERRAIN_CHUNK_SIZE as i32))
                .as_vec2()
                * VOXEL_SIZE;

            // Altitude
            let noise_val = terrain_generator.get([world_xz.x as f64, world_xz.y as f64]);
            let altitude = (noise_val * 100.0) as i32;
            altitude_map[AltitudeMapShape {}.linearize([x, z]) as usize] = altitude;

            // Biome
            let biome = config.get_biome(world_xz.x as f64, world_xz.y as f64, altitude);
            biome_map[AltitudeMapShape {}.linearize([x, z]) as usize] = biome.id;
        }
    }

    (altitude_map, biome_map)
}

pub fn generate_base_terrain(
    chunk_pos: IVec3,
    altitude_map: &[i32],
    biome_map: &[u8],
    config: &BiomeRegistry,
) -> TerrainChunkData {
    let mut chunk_data = TerrainChunkData::new_empty(chunk_pos);

    for z in 0..TERRAIN_CHUNK_SIZE {
        for x in 0..TERRAIN_CHUNK_SIZE {
            let idx = AltitudeMapShape {}.linearize([x, z]) as usize;
            let global_altitude = altitude_map[idx];
            let biome_id = biome_map[idx];
            let biome = config.get_biome_by_id(biome_id);
            let altitude = global_altitude - chunk_pos.y * TERRAIN_CHUNK_SIZE as i32;

            for y in 0..TERRAIN_CHUNK_SIZE {
                let local_y = y as i32;
                let world_y = local_y + chunk_pos.y * TERRAIN_CHUNK_SIZE as i32;
                let voxel = if local_y < altitude - 3 {
                    Voxel::STONE
                } else if local_y < altitude {
                    biome.sub_surface_block
                } else if local_y == altitude {
                    biome.surface_block
                } else if world_y < 0 {
                    Voxel::WATER
                } else {
                    Voxel::EMPTY
                };

                if voxel != Voxel::EMPTY {
                    *chunk_data.get_local_at_mut(UVec3::new(x, y, z)) = voxel;
                }
            }
        }
    }
    chunk_data
}

pub fn generate_features(
    chunk_pos: IVec3,
    seed: u32,
    altitude_map: &[i32],
    config: &BiomeRegistry,
) -> Vec<(IVec3, Voxel)> {
    let mut changes = Vec::new();
    
    for z in 0..TERRAIN_CHUNK_SIZE {
        for x in 0..TERRAIN_CHUNK_SIZE {
            let idx = AltitudeMapShape{}.linearize([x, z]) as usize;
            let altitude = altitude_map[idx];
            
            let local_y = altitude - chunk_pos.y * TERRAIN_CHUNK_SIZE as i32;
            
            if local_y >= 0 && local_y < TERRAIN_CHUNK_SIZE as i32 {
                let world_x = x as i32 + chunk_pos.x * TERRAIN_CHUNK_SIZE as i32;
                let world_z = z as i32 + chunk_pos.z * TERRAIN_CHUNK_SIZE as i32;
                let biome = config.get_biome(world_x as f64, world_z as f64, altitude);
                
                if !biome.features.is_empty() {
                    let prob_hash = feature::hash(world_x, world_z, seed);
                    let prob = (prob_hash % 10000) as f32 / 10000.0;
                    
                    if prob < biome.feature_probability {
                        let feature_idx = (prob_hash as usize) % biome.features.len();
                        let feature = &biome.features[feature_idx];
                        
                        // Place feature at (world_x, altitude + 1, world_z)
                        let origin = IVec3::new(world_x, altitude + 1, world_z);
                        let feature_changes = feature.place(origin, seed);
                        changes.extend(feature_changes);
                    }
                }
            }
        }
    }
    changes
}
