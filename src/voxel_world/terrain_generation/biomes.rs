use bevy::platform::collections::HashMap;
use noise::{NoiseFn, Perlin};
use std::sync::Arc;
use crate::voxel_world::{terrain_generation::feature::BigOakTreeFeature, voxel::Voxel};
use super::feature::{Feature, OakTreeFeature, CactusFeature, FlowerFeature, PineTreeFeature, BirchTreeFeature, IceSpikeFeature, BambooFeature, AcaciaTreeFeature, JungleTreeFeature, MegaJungleTreeFeature, JungleBushFeature};

pub struct Biome {
    pub id: u8,
    #[allow(dead_code)]
    pub name: &'static str,
    pub surface_block: Voxel,
    pub sub_surface_block: Voxel,
    pub features: Vec<(Arc<dyn Feature>, f32)>,
}

pub struct BiomeRegistry {
    // Using a simple grid or logic for now, but storing biomes by ID
    pub biomes: HashMap<u8, Biome>,
    temperature_noise: Box<dyn NoiseFn<f64, 2> + Send + Sync>,
    humidity_noise: Box<dyn NoiseFn<f64, 2> + Send + Sync>,
    rarity_noise: Box<dyn NoiseFn<f64, 2> + Send + Sync>,
}

impl BiomeRegistry {
    pub fn new(seed: u32) -> Self {
        let mut biomes = HashMap::new();

        // Plains
        biomes.insert(0, Biome {
            id: 0,
            name: "Plains",
            surface_block: Voxel::GRASS,
            sub_surface_block: Voxel::DIRT,
            features: vec![(Arc::new(BigOakTreeFeature), 0.0003)],
        });

        // Desert
        biomes.insert(1, Biome {
            id: 1,
            name: "Desert",
            surface_block: Voxel::SAND,
            sub_surface_block: Voxel::SAND,
            features: vec![(Arc::new(CactusFeature), 0.01)],
        });

        // Mountains
        biomes.insert(2, Biome {
            id: 2,
            name: "Mountains",
            surface_block: Voxel::STONE,
            sub_surface_block: Voxel::STONE,
            features: vec![],
        });

        // Snow
        biomes.insert(3, Biome {
            id: 3,
            name: "Snow",
            surface_block: Voxel::SNOW,
            sub_surface_block: Voxel::DIRT,
            features: vec![(Arc::new(PineTreeFeature), 0.02)],
        });

        // Ocean
        biomes.insert(4, Biome {
            id: 4,
            name: "Ocean",
            // Yが低い場所では水ブロックを生成するようにするので, ここでは砂利にしておく
            surface_block: Voxel::GRAVEL,
            sub_surface_block: Voxel::STONE,
            features: vec![],
        });

        // Oak Forest
        biomes.insert(5, Biome {
            id: 5,
            name: "Oak Forest",
            surface_block: Voxel::GRASS,
            sub_surface_block: Voxel::DIRT,
            features: vec![
                (Arc::new(OakTreeFeature), 0.01),
                (Arc::new(FlowerFeature), 0.01)
            ],
        });

        // Birch Forest
        biomes.insert(6, Biome {
            id: 6,
            name: "Birch Forest",
            surface_block: Voxel::GRASS,
            sub_surface_block: Voxel::DIRT,
            features: vec![
                (Arc::new(BirchTreeFeature), 0.01),
                (Arc::new(FlowerFeature), 0.01)
            ],
        });

        // Flower Field
        biomes.insert(7, Biome {
            id: 7,
            name: "Flower Field",
            surface_block: Voxel::GRASS,
            sub_surface_block: Voxel::DIRT,
            features: vec![(Arc::new(FlowerFeature), 0.1)],
        });

        // Snow Field
        biomes.insert(8, Biome {
            id: 8,
            name: "Snow Field",
            surface_block: Voxel::SNOW,
            sub_surface_block: Voxel::SNOW,
            features: vec![],
        });

        // Savanna
        biomes.insert(9, Biome {
            id: 9,
            name: "Savanna",
            surface_block: Voxel::GRASS,
            sub_surface_block: Voxel::DIRT,
            features: vec![(Arc::new(AcaciaTreeFeature), 0.002)],
        });

        // Jungle
        biomes.insert(10, Biome {
            id: 10,
            name: "Jungle",
            surface_block: Voxel::GRASS,
            sub_surface_block: Voxel::DIRT,
            features: vec![
                (Arc::new(MegaJungleTreeFeature), 0.002),
                (Arc::new(JungleTreeFeature), 0.02),
                (Arc::new(JungleBushFeature), 0.05),
                (Arc::new(FlowerFeature), 0.01)
            ],
        });

        // Beach
        biomes.insert(11, Biome {
            id: 11,
            name: "Beach",
            surface_block: Voxel::SAND,
            sub_surface_block: Voxel::SAND,
            features: vec![],
        });

        // Cold Ocean
        biomes.insert(12, Biome {
            id: 12,
            name: "Cold Ocean",
            surface_block: Voxel::GRAVEL,
            sub_surface_block: Voxel::STONE,
            features: vec![],
        });

        // Sunflower Plains
        biomes.insert(13, Biome {
            id: 13,
            name: "Sunflower Plains",
            surface_block: Voxel::GRASS,
            sub_surface_block: Voxel::DIRT,
            features: vec![
                (Arc::new(BigOakTreeFeature), 0.001),
                (Arc::new(FlowerFeature), 0.2)
            ],
        });

        // Ice Spikes
        biomes.insert(14, Biome {
            id: 14,
            name: "Ice Spikes",
            surface_block: Voxel::SNOW,
            sub_surface_block: Voxel::PACKED_ICE,
            features: vec![(Arc::new(IceSpikeFeature), 0.02)],
        });

        // Red Desert
        biomes.insert(15, Biome {
            id: 15,
            name: "Red Desert",
            surface_block: Voxel::RED_SAND,
            sub_surface_block: Voxel::RED_SAND,
            features: vec![(Arc::new(CactusFeature), 0.01)],
        });

        // Bamboo Jungle
        biomes.insert(16, Biome {
            id: 16,
            name: "Bamboo Jungle",
            surface_block: Voxel::GRASS,
            sub_surface_block: Voxel::DIRT,
            features: vec![
                (Arc::new(BambooFeature), 0.1),
                (Arc::new(JungleTreeFeature), 0.005),
                (Arc::new(JungleBushFeature), 0.01)
            ],
        });

        Self {
            biomes,
            temperature_noise: Box::new(Perlin::new(seed + 100)),
            humidity_noise: Box::new(Perlin::new(seed + 200)),
            rarity_noise: Box::new(Perlin::new(seed + 300)),
        }
    }

    pub fn get_biome_by_id(&self, id: u8) -> &Biome {
        self.biomes.get(&id).unwrap_or_else(|| self.biomes.get(&0).unwrap())
    }

    pub fn get_biome(&self, x: f64, z: f64, altitude: i32) -> &Biome {
        let temp = self.temperature_noise.get([x * 0.0004, z * 0.0004])
            - (altitude - 20) as f64 * 0.01;

        let humidity = self.humidity_noise.get([x * 0.0004, z * 0.0004])
            + temp * 0.2;

        let rarity = self.rarity_noise.get([x * 0.002, z * 0.002]);

        let id = if altitude >= -4 && altitude < 2 && temp > -0.2 {
                11 // Beach
        } else if altitude < 0 {
            if temp < -0.5 {
                12 // Cold Ocean
            } else {
                4 // Ocean
            }
        } else if temp < -0.4 {
            if humidity < 0.0 {
                if rarity > 0.6 { 14 } else { 8 } // Ice Spikes or Snow Field
            } else {
                3 // Snow (Pine Forest)
            }
        } else if temp < 0.4 {
            if humidity < -0.7 {
                2 // Mountains
            } else if humidity < -0.4 {
                if rarity > 0.5 { 13 } else { 0 } // Sunflower Plains or Plains
            } else if humidity < 0.0 {
                6 // Birch Forest
            } else if humidity < 0.3 {
                if rarity > 0.4 { 7 } else { 5 } // Flower Field or Oak Forest
            } else {
                5 // Oak Forest
            }
        } else {
            if humidity < -0.2 {
                if rarity > 0.5 { 15 } else { 1 } // Red Desert or Desert
            } else if humidity < 0.3 {
                9 // Savanna
            } else {
                if rarity > 0.5 { 16 } else { 10 } // Bamboo Jungle or Jungle
            }
        };

        self.biomes.get(&id).unwrap_or_else(|| self.biomes.get(&0).unwrap())
    }
}
