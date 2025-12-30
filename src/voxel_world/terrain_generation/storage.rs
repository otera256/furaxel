use bevy::{platform::collections::HashMap, prelude::*};

#[derive(Debug, Default, Resource)]
pub(super) struct TerrainGenerationStorage {
    pub altitude_maps: HashMap<IVec2, Box<[i32]>>,
}