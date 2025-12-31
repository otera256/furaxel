use bevy::platform::collections::HashMap;
use noise::{NoiseFn, Perlin};
use std::sync::Arc;
use crate::voxel_world::{terrain_generation::feature::BigOakTreeFeature, voxel::Voxel};
use super::feature::{Feature, OakTreeFeature, CactusFeature, FlowerFeature, PineTreeFeature, BirchTreeFeature};

pub struct Biome {
    pub id: u8,
    #[allow(dead_code)]
    pub name: &'static str,
    pub surface_block: Voxel,
    pub sub_surface_block: Voxel,
    pub features: Vec<Arc<dyn Feature>>,
    pub feature_probability: f32,
}

pub struct BiomeRegistry {
    // Using a simple grid or logic for now, but storing biomes by ID
    pub biomes: HashMap<u8, Biome>,
    temperature_noise: Box<dyn NoiseFn<f64, 2> + Send + Sync>,
    humidity_noise: Box<dyn NoiseFn<f64, 2> + Send + Sync>,
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
            features: vec![Arc::new(BigOakTreeFeature)],
            feature_probability: 0.001,
        });

        // Desert
        biomes.insert(1, Biome {
            id: 1,
            name: "Desert",
            surface_block: Voxel::SAND,
            sub_surface_block: Voxel::SAND,
            features: vec![Arc::new(CactusFeature)],
            feature_probability: 0.01,
        });

        // Mountains
        biomes.insert(2, Biome {
            id: 2,
            name: "Mountains",
            surface_block: Voxel::STONE,
            sub_surface_block: Voxel::STONE,
            features: vec![],
            feature_probability: 0.0,
        });

        // Snow
        biomes.insert(3, Biome {
            id: 3,
            name: "Snow",
            surface_block: Voxel::SNOW,
            sub_surface_block: Voxel::DIRT,
            features: vec![Arc::new(PineTreeFeature)],
            feature_probability: 0.02,
        });

        // Ocean
        biomes.insert(4, Biome {
            id: 4,
            name: "Ocean",
            // Yが低い場所では水ブロックを生成するようにするので, ここでは砂利にしておく
            surface_block: Voxel::GRAVEL,
            sub_surface_block: Voxel::STONE,
            features: vec![],
            feature_probability: 0.0,
        });

        // Oak Forest
        biomes.insert(5, Biome {
            id: 5,
            name: "Oak Forest",
            surface_block: Voxel::GRASS,
            sub_surface_block: Voxel::DIRT,
            features: vec![Arc::new(OakTreeFeature), Arc::new(FlowerFeature)],
            feature_probability: 0.02,
        });

        // Birch Forest
        biomes.insert(6, Biome {
            id: 6,
            name: "Birch Forest",
            surface_block: Voxel::GRASS,
            sub_surface_block: Voxel::DIRT,
            features: vec![Arc::new(BirchTreeFeature), Arc::new(FlowerFeature)],
            feature_probability: 0.02,
        });

        // Flower Field
        biomes.insert(7, Biome {
            id: 7,
            name: "Flower Field",
            surface_block: Voxel::GRASS,
            sub_surface_block: Voxel::DIRT,
            features: vec![Arc::new(FlowerFeature)],
            feature_probability: 0.1,
        });

        // Snow Field
        biomes.insert(8, Biome {
            id: 8,
            name: "Snow Field",
            surface_block: Voxel::SNOW,
            sub_surface_block: Voxel::SNOW,
            features: vec![],
            feature_probability: 0.0,
        });

        // Savanna
        biomes.insert(9, Biome {
            id: 9,
            name: "Savanna",
            surface_block: Voxel::GRASS,
            sub_surface_block: Voxel::DIRT,
            features: vec![Arc::new(OakTreeFeature)],
            feature_probability: 0.01,
        });

        // Jungle
        biomes.insert(10, Biome {
            id: 10,
            name: "Jungle",
            surface_block: Voxel::GRASS,
            sub_surface_block: Voxel::DIRT,
            features: vec![Arc::new(OakTreeFeature), Arc::new(BirchTreeFeature), Arc::new(FlowerFeature)],
            feature_probability: 0.08,
        });

        Self {
            biomes,
            temperature_noise: Box::new(Perlin::new(seed + 100)),
            humidity_noise: Box::new(Perlin::new(seed + 200)),
        }
    }

    pub fn get_biome_by_id(&self, id: u8) -> &Biome {
        self.biomes.get(&id).unwrap_or_else(|| self.biomes.get(&0).unwrap())
    }

    pub fn get_biome(&self, x: f64, z: f64, altitude: i32) -> &Biome {
        let temp = self.temperature_noise.get([x * 0.001, z * 0.001])
            - (altitude - 20) as f64 * 0.01;

        let humidity = self.humidity_noise.get([x * 0.001, z * 0.001])
            + temp * 0.2;

        let id = if altitude < 0 {
            4 // Ocean
        } else if temp < -0.4 {
            if humidity < 0.0 {
                8 // Snow Field
            } else {
                3 // Snow (Pine Forest)
            }
        } else if temp < 0.5 {
            if humidity < -0.7 {
                2 // Mountains
            } else if humidity < -0.2 {
                0 // Plains
            } else if humidity < 0.2 {
                6 // Birch Forest
            } else if humidity < 0.5 {
                7 // Flower Field
            } else {
                5 // Oak Forest
            }
        } else {
            if humidity < -0.2 {
                1 // Desert
            } else if humidity < 0.3 {
                9 // Savanna
            } else {
                10 // Jungle
            }
        };

        self.biomes.get(&id).unwrap_or_else(|| self.biomes.get(&0).unwrap())
    }
}
