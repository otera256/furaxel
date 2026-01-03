use bevy::platform::collections::HashMap;
use std::sync::Arc;
use crate::voxel_world::{pipelines::cpu_noise::feature::BigOakTreeFeature, core::voxel::Voxel};
use super::feature::{Feature, OakTreeFeature, CactusFeature, FlowerFeature, PineTreeFeature, BirchTreeFeature, IceSpikeFeature, BambooFeature, AcaciaTreeFeature, JungleTreeFeature, MegaJungleTreeFeature, JungleBushFeature};

pub struct BiomeData {
    pub id: u8,
    #[allow(dead_code)]
    pub name: &'static str,
    pub surface_block: Voxel,
    pub sub_surface_block: Voxel,
    pub features: Vec<(Arc<dyn Feature>, f32)>,
}

macro_rules! define_biomes {
    (
        $(
            $name:ident = $id:expr => {
                name: $biome_name:expr,
                surface: $surface:expr,
                sub_surface: $sub:expr,
                features: $features:expr
            }
        ),* $(,)?
    ) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct Biome {
            pub id: u8,
        }

        impl Biome {
            $(
                pub const $name: Self = Self { id: $id };
            )*
            
            pub fn new(id: u8) -> Self {
                Self { id }
            }
        }

        pub fn get_biome_definitions() -> Vec<BiomeData> {
            vec![
                $(
                    BiomeData {
                        id: $id,
                        name: $biome_name,
                        surface_block: $surface,
                        sub_surface_block: $sub,
                        features: $features,
                    },
                )*
            ]
        }
    }
}

define_biomes! {
    PLAINS = 0 => {
        name: "Plains",
        surface: Voxel::GRASS,
        sub_surface: Voxel::DIRT,
        features: vec![(Arc::new(BigOakTreeFeature), 0.0001)]
    },
    DESERT = 1 => {
        name: "Desert",
        surface: Voxel::SAND,
        sub_surface: Voxel::SAND,
        features: vec![(Arc::new(CactusFeature), 0.01)]
    },
    MOUNTAINS = 2 => {
        name: "Mountains",
        surface: Voxel::STONE,
        sub_surface: Voxel::STONE,
        features: vec![]
    },
    SNOW = 3 => {
        name: "Snow",
        surface: Voxel::SNOW,
        sub_surface: Voxel::DIRT,
        features: vec![(Arc::new(PineTreeFeature), 0.02)]
    },
    OCEAN = 4 => {
        name: "Ocean",
        surface: Voxel::GRAVEL,
        sub_surface: Voxel::STONE,
        features: vec![]
    },
    OAK_FOREST = 5 => {
        name: "Oak Forest",
        surface: Voxel::GRASS,
        sub_surface: Voxel::DIRT,
        features: vec![
            (Arc::new(OakTreeFeature), 0.02),
            (Arc::new(FlowerFeature), 0.02)
        ]
    },
    BIRCH_FOREST = 6 => {
        name: "Birch Forest",
        surface: Voxel::GRASS,
        sub_surface: Voxel::DIRT,
        features: vec![
            (Arc::new(BirchTreeFeature), 0.02),
            (Arc::new(FlowerFeature), 0.02)
        ]
    },
    FLOWER_FIELD = 7 => {
        name: "Flower Field",
        surface: Voxel::GRASS,
        sub_surface: Voxel::DIRT,
        features: vec![(Arc::new(FlowerFeature), 0.3)]
    },
    SNOW_FIELD = 8 => {
        name: "Snow Field",
        surface: Voxel::SNOW,
        sub_surface: Voxel::SNOW,
        features: vec![]
    },
    SAVANNA = 9 => {
        name: "Savanna",
        surface: Voxel::GRASS,
        sub_surface: Voxel::DIRT,
        features: vec![(Arc::new(AcaciaTreeFeature), 0.002)]
    },
    JUNGLE = 10 => {
        name: "Jungle",
        surface: Voxel::GRASS,
        sub_surface: Voxel::DIRT,
        features: vec![
            (Arc::new(MegaJungleTreeFeature), 0.005),
            (Arc::new(JungleTreeFeature), 0.03),
            (Arc::new(JungleBushFeature), 0.05),
            (Arc::new(FlowerFeature), 0.01)
        ]
    },
    BEACH = 11 => {
        name: "Beach",
        surface: Voxel::SAND,
        sub_surface: Voxel::SAND,
        features: vec![]
    },
    COLD_OCEAN = 12 => {
        name: "Cold Ocean",
        surface: Voxel::GRAVEL,
        sub_surface: Voxel::STONE,
        features: vec![]
    },
    SUNFLOWER_PLAINS = 13 => {
        name: "Sunflower Plains",
        surface: Voxel::GRASS,
        sub_surface: Voxel::DIRT,
        features: vec![
            (Arc::new(BigOakTreeFeature), 0.001),
            (Arc::new(FlowerFeature), 0.2)
        ]
    },
    ICE_SPIKES = 14 => {
        name: "Ice Spikes",
        surface: Voxel::SNOW,
        sub_surface: Voxel::PACKED_ICE,
        features: vec![(Arc::new(IceSpikeFeature), 0.01)]
    },
    RED_DESERT = 15 => {
        name: "Red Desert",
        surface: Voxel::RED_SAND,
        sub_surface: Voxel::RED_SAND,
        features: vec![(Arc::new(CactusFeature), 0.01)]
    },
    BAMBOO_JUNGLE = 16 => {
        name: "Bamboo Jungle",
        surface: Voxel::GRASS,
        sub_surface: Voxel::DIRT,
        features: vec![
            (Arc::new(BambooFeature), 0.1),
            (Arc::new(JungleTreeFeature), 0.005),
            (Arc::new(JungleBushFeature), 0.01)
        ]
    }
}

pub struct BiomeRegistry {
    // Using a simple grid or logic for now, but storing biomes by ID
    pub biomes: HashMap<u8, BiomeData>,
}

impl BiomeRegistry {
    pub fn new(_seed: u32) -> Self {
        let mut biomes = HashMap::new();
        for biome_data in get_biome_definitions() {
            biomes.insert(biome_data.id, biome_data);
        }
        Self { biomes }
    }

    pub fn get_biome_data(&self, biome: Biome) -> &BiomeData {
        self.biomes.get(&biome.id).unwrap_or_else(|| self.biomes.get(&0).unwrap())
    }
    
    pub fn get_biome_data_by_id(&self, id: u8) -> &BiomeData {
        self.biomes.get(&id).unwrap_or_else(|| self.biomes.get(&0).unwrap())
    }

    pub fn resolve_biome(&self, temp: f64, humidity: f64, rarity: f64, altitude: i32) -> Biome {
        if altitude >= -3 && altitude < 0 && temp > -0.1 {
            return Biome::BEACH;
        } else if altitude < 0 {
            if temp < -0.3 {
                return Biome::COLD_OCEAN;
            } else {
                return Biome::OCEAN;
            }
        } else if temp < -0.3 {
            if humidity < 0.0 {
                if rarity > 0.3 { return Biome::ICE_SPIKES; } else { return Biome::SNOW_FIELD; }
            } else {
                return Biome::SNOW;
            }
        } else if temp < 0.2 {
            if humidity < -0.3 {
                return Biome::MOUNTAINS;
            } else if humidity < -0.1 {
                if rarity > 0.3 { return Biome::SUNFLOWER_PLAINS; } else { return Biome::PLAINS; }
            } else if humidity < 0.0 {
                return Biome::BIRCH_FOREST;
            } else if humidity < 0.3 {
                if rarity > 0.3 { return Biome::FLOWER_FIELD; } else { return Biome::OAK_FOREST; }
            } else {
                return Biome::OAK_FOREST;
            }
        } else {
            if humidity < -0.15 {
                if rarity > 0.3 { return Biome::RED_DESERT; } else { return Biome::DESERT; }
            } else if humidity < 0.15 {
                return Biome::SAVANNA;
            } else {
                if rarity > 0.3 { return Biome::BAMBOO_JUNGLE; } else { return Biome::JUNGLE; }
            }
        }
    }
}
