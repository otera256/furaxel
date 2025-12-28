use bevy::{prelude::*, math::IVec3, platform::collections::{HashMap, HashSet}};

#[derive(Resource, Debug, Default)]
pub struct ChunkEntities {
    pub entities: HashMap<IVec3, Entity>,
    to_create: HashSet<IVec3>,
    to_remove: HashSet<IVec3>,
}

#[derive(Resource, Debug, Clone, Copy)]
pub struct RenderDistanceParams {
    pub player_chunk: IVec3,
    horizontal: i32,
    vertical: i32,
}

impl Default for RenderDistanceParams {
    fn default() -> Self {
        Self {
            player_chunk: IVec3::ZERO,
            horizontal: 8,
            vertical: 3,
        }
    }
}

