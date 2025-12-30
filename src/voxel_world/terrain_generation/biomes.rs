use bevy::platform::collections::HashMap;
use noise::{NoiseFn, Perlin};
use std::sync::Arc;
use crate::voxel_world::voxel::Voxel;
use super::feature::{Feature, TreeFeature};

pub struct Biome {
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
            name: "Plains",
            surface_block: Voxel::GRASS,
            sub_surface_block: Voxel::DIRT,
            features: vec![Arc::new(TreeFeature)],
            feature_probability: 0.01,
        });

        // Desert
        biomes.insert(1, Biome {
            name: "Desert",
            surface_block: Voxel::SAND,
            sub_surface_block: Voxel::SAND,
            features: vec![],
            feature_probability: 0.0,
        });

        // Mountains
        biomes.insert(2, Biome {
            name: "Mountains",
            surface_block: Voxel::STONE,
            sub_surface_block: Voxel::STONE,
            features: vec![],
            feature_probability: 0.0,
        });

        // Snow
        biomes.insert(3, Biome {
            name: "Snow",
            surface_block: Voxel::SNOW,
            sub_surface_block: Voxel::DIRT,
            features: vec![Arc::new(TreeFeature)],
            feature_probability: 0.005,
        });

        Self {
            biomes,
            temperature_noise: Box::new(Perlin::new(seed + 100)),
            humidity_noise: Box::new(Perlin::new(seed + 200)),
        }
    }

    pub fn get_biome(&self, x: f64, z: f64) -> &Biome {
        let temp = self.temperature_noise.get([x * 0.001, z * 0.001]);
        let humidity = self.humidity_noise.get([x * 0.001, z * 0.001]);

        let id = if temp > 0.5 {
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
