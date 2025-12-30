use std::sync::Arc;
use bevy::{platform::collections::{HashMap, HashSet}, prelude::*};

#[derive(Debug, Default, Resource)]
pub(super) struct TerrainGenerationStorage {
    pub altitude_maps: HashMap<IVec2, Arc<[i32]>>,
    pub biome_maps: HashMap<IVec2, Arc<[u8]>>,
    pub base_terrain_generated: HashSet<IVec3>,
    pub fully_generated: HashSet<IVec3>,
}