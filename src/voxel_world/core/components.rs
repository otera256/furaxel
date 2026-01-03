use bevy::{prelude::*, platform::collections::HashMap};
use super::coordinates::TERRAIN_CHUNK_SIZE;

#[derive(Component, Debug, Clone, Copy)]
pub struct TerrainChunk {
    pub position: IVec3,
}

#[allow(dead_code)]
impl TerrainChunk {
    pub fn chunk_origin(&self) -> IVec3 {
        self.position * TERRAIN_CHUNK_SIZE as i32
    }
    pub fn chunk_origin_f32(&self) -> Vec3 {
        self.chunk_origin().as_vec3() * (TERRAIN_CHUNK_SIZE as f32)
    }
}

#[derive(Resource, Debug, Default)]
pub struct ChunkEntities {
    pub entities: HashMap<IVec3, Entity>,
}

#[derive(Resource, Debug, Clone, Copy)]
pub struct RenderDistanceParams {
    pub player_chunk: IVec3,
    pub horizontal: i32,
    pub vertical: i32,
}

impl Default for RenderDistanceParams {
    fn default() -> Self {
        Self {
            player_chunk: IVec3::ZERO,
            horizontal: 16,
            vertical: 4,
        }
    }
}