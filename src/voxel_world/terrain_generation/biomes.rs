use bevy::platform::collections::HashMap;
use noise::{NoiseFn, Perlin};
use std::sync::Arc;
use crate::voxel_world::voxel::Voxel;
use super::feature::{Feature, TreeFeature, CactusFeature, FlowerFeature, PineTreeFeature, BirchTreeFeature};

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
            features: vec![
                Arc::new(TreeFeature),
                Arc::new(BirchTreeFeature),
                Arc::new(FlowerFeature),
                Arc::new(FlowerFeature),
                Arc::new(FlowerFeature),
            ],
            feature_probability: 0.05,
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
            features: vec![Arc::new(PineTreeFeature)],
            feature_probability: 0.02,
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
        let temp = self.temperature_noise.get([x * 0.001, z * 0.001]);
        let humidity = self.humidity_noise.get([x * 0.001, z * 0.001]);

        let id = if altitude < 0 {
            4 // Ocean
        } else if temp > 0.5 {
            if humidity < -0.2 {
                1 // Desert
            } else {
                0 // Plains
            }
        } else if temp < -0.5 {
            3 // Snow
        } else {
            if humidity > 0.5 {
                0 // Plains (Forest-like)
            } else {
                2 // Mountains (using as rocky/hills for now)
            }
        };

        self.biomes.get(&id).unwrap_or_else(|| self.biomes.get(&0).unwrap())
    }
}
