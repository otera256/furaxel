use bevy::prelude::*;
use block_mesh::ndshape::{AbstractShape, ConstShape2u32};
use noise::{Fbm, MultiFractal, NoiseFn, OpenSimplex, RidgedMulti};
use crate::voxel_world::{
    terrain_chunk::{TERRAIN_CHUNK_SIZE, TerrainChunkData},
    voxel::{VOXEL_SIZE, Voxel},
};
use super::{biomes::BiomeRegistry, feature};

type AltitudeMapShape = ConstShape2u32<TERRAIN_CHUNK_SIZE, TERRAIN_CHUNK_SIZE>;

fn spline_interp(x: f64, min_x: f64, max_x: f64, min_y: f64, max_y: f64) -> f64 {
    let t = ((x - min_x) / (max_x - min_x)).clamp(0.0, 1.0);
    let t_smooth = t * t * (3.0 - 2.0 * t);
    min_y + t_smooth * (max_y - min_y)
}

pub fn generate_altitude_map(
    seed: u32,
    chunk_xz: IVec2,
    config: &BiomeRegistry,
) -> (Vec<i32>, Vec<u8>) {
    // --- Noise Generation Logic ---
    // Domain Warping: Used to distort the coordinate system
    let domain_warp = Fbm::<OpenSimplex>::new(seed.wrapping_add(999))
        .set_frequency(0.02)
        .set_octaves(4)
        .set_persistence(0.5);

    // Continentalness: Controls the general height (Ocean, Coast, Land, Inland)
    let continentalness = Fbm::<OpenSimplex>::new(seed)
        .set_frequency(0.002)
        .set_octaves(6)
        .set_persistence(0.5);

    // Erosion: Controls the roughness/flatness
    let erosion = Fbm::<OpenSimplex>::new(seed.wrapping_add(1))
        .set_frequency(0.01)
        .set_octaves(6)
        .set_persistence(0.5);

    // Peaks & Valleys: Adds local detail (mountains, hills)
    let peaks_valleys = RidgedMulti::<OpenSimplex>::new(seed.wrapping_add(2))
        .set_frequency(0.01)
        .set_octaves(8);

    // Temperature: Controls biome temperature
    let temperature = Fbm::<OpenSimplex>::new(seed.wrapping_add(100))
        .set_frequency(0.0015)
        .set_octaves(4);
    // Humidity: Controls biome humidity
    let humidity = Fbm::<OpenSimplex>::new(seed.wrapping_add(200))
        .set_frequency(0.0015)
        .set_octaves(4);

    // Rarity: Controls rare biome variants
    let rarity = Fbm::<OpenSimplex>::new(seed.wrapping_add(300))
        .set_frequency(0.001)
        .set_octaves(4);

    let mut altitude_map = vec![0i32; (TERRAIN_CHUNK_SIZE * TERRAIN_CHUNK_SIZE) as usize];
    let mut biome_map = vec![0u8; (TERRAIN_CHUNK_SIZE * TERRAIN_CHUNK_SIZE) as usize];

    for z in 0..TERRAIN_CHUNK_SIZE {
        for x in 0..TERRAIN_CHUNK_SIZE {
            let world_xz = (IVec2::new(x as i32, z as i32)
                + chunk_xz * IVec2::splat(TERRAIN_CHUNK_SIZE as i32))
                .as_vec2()
                * VOXEL_SIZE;

            // Domain Warping
            let warp_strength = 50.0;
            let wx = domain_warp.get([world_xz.x as f64, world_xz.y as f64]) * warp_strength;
            let wz = domain_warp.get([world_xz.x as f64 + 500.0, world_xz.y as f64 + 500.0]) * warp_strength;

            let warped_x = world_xz.x as f64 + wx;
            let warped_z = world_xz.y as f64 + wz;

            let raw_c = continentalness.get([warped_x, warped_z]);
            let raw_e = erosion.get([warped_x, warped_z]);
            let raw_pv = peaks_valleys.get([warped_x, warped_z]);
            let raw_temp = temperature.get([warped_x, warped_z]);
            let raw_hum = humidity.get([warped_x, warped_z]);
            let raw_rarity = rarity.get([warped_x, warped_z]);

            // 相互作用1: 気温が侵食に影響を与える
            // 気温が高いほど風化が進みやすく、地形が平坦になりやすいと仮定します。
            // raw_tempが高いほどerosionの値が大きくなり、平坦化係数が強まります。
            let erosion_modified = raw_e + raw_temp * 0.15;

            // 相互作用2: 湿度が山岳の形状に影響を与える
            // 湿度が高い場所では水による浸食で谷が深くなり、結果として起伏が激しくなると仮定します。
            // raw_humが高いほどpeaks_valleysの影響を強めます。
            let pv_modified = raw_pv * (1.0 + raw_hum.max(0.0) * 0.3);

            // Continentalness (大陸性) による基本高度の計算
            // 海、海岸、平野、山岳といった大まかな地形を決定します。
            let (min_c, max_c, min_h, max_h) = if raw_c < -0.3 {
                (-1.0, -0.3, -100.0, -15.0) // 深海
            } else if raw_c < -0.1 {
                (-0.3, -0.1, -15.0, -5.0)  // 浅瀬
            } else if raw_c < 0.1 {
                (-0.1, 0.1, -5.0, 5.0)     // 海岸
            } else if raw_c < 0.2 {
                (0.1, 0.2, 5.0, 20.0)      // 平野
            } else if raw_c < 0.25 {
                (0.2, 0.25, 20.0, 60.0)     // 丘陵
            } else if raw_c < 0.3 {
                (0.25, 0.3, 60.0, 80.0)     // 高原
            } else {
                (0.3, 1.0, 80.0, 1000.0)    // 山岳
            };

            let mut height = spline_interp(raw_c, min_c, max_c, min_h, max_h);

            // Erosion (侵食) による地形の平坦化
            // 値が大きいほど侵食が進んでおり、地形が滑らかになります。
            let (min_e, max_e, min_f, max_f) = if erosion_modified < -0.5 {
                (-1.0, -0.5, 2.5, 1.0) // 侵食が少ない（険しい）
            } else if erosion_modified < 0.0 {
                (-0.5, 0.0, 1.0, 0.3)
            } else if erosion_modified < 0.5 {
                (0.0, 0.5, 0.3, 0.1)
            } else {
                (0.5, 1.0, 0.1, 0.05) // 侵食が多い（平坦）
            };
            
            let erosion_factor = spline_interp(erosion_modified, min_e, max_e, min_f, max_f);

            // Peaks & Valleys (山谷) による詳細な起伏の追加
            // 侵食係数を掛けることで、平坦な場所では起伏を抑えます。
            // また、大陸性が高い（内陸）ほど山が高くなるように補正をかけます。
            height += pv_modified * erosion_factor * spline_interp(raw_c, 0.0, 1.0, 3.0, 300.0);
            
            let altitude = height as i32;
            altitude_map[AltitudeMapShape {}.linearize([x, z]) as usize] = altitude;

            // 相互作用3: 高度が気温に影響を与える (気温減率)
            // 標高が高いほど気温は下がります。
            let temp_final = raw_temp - (altitude - 20) as f64 * 0.005;

            // 相互作用4: 気温が湿度に影響を与える
            // 気温が高いと飽和水蒸気量が増えるため、相対的な湿度の感じ方が変わりますが、
            // ここでは「暖かい空気は水分を多く含む」として湿度を少し上げます。
            let humidity_final = raw_hum + temp_final * 0.1;

            let biome = config.resolve_biome(temp_final, humidity_final, raw_rarity, altitude);
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
            let biome = config.get_biome_data_by_id(biome_id);
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
                    if biome.id == 12 && world_y == -1 {
                        Voxel::ICE
                    } else {
                        Voxel::WATER
                    }
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
    biome_map: &[u8],
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
                
                let biome_id = biome_map[idx];
                let biome = config.get_biome_data_by_id(biome_id);
                
                if !biome.features.is_empty() {
                    for (i, (feature, probability)) in biome.features.iter().enumerate() {
                        let prob_hash = feature::hash(world_x, world_z, seed.wrapping_add(i as u32));
                        let prob = (prob_hash % 10000) as f32 / 10000.0;
                        
                        if prob < *probability {
                            // Place feature at (world_x, altitude + 1, world_z)
                            let origin = IVec3::new(world_x, altitude + 1, world_z);
                            let feature_changes = feature.place(origin, seed);
                            changes.extend(feature_changes);
                        }
                    }
                }
            }
        }
    }
    changes
}
