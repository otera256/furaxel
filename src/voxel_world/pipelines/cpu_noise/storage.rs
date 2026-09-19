use std::sync::Arc;
use bevy::{platform::collections::HashMap, prelude::*};

#[derive(Debug, Default, Resource)]
pub struct TerrainGenerationStorage {
    pub altitude_maps: HashMap<IVec2, Arc<[i32]>>,
    pub biome_maps: HashMap<IVec2, Arc<[u8]>>,
}
