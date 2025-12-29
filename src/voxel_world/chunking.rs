use bevy::{prelude::*, math::IVec3, platform::collections::HashMap};
use itertools::iproduct;

use crate::voxel_world::{terrain_chunk::{TERRAIN_CHUNK_LENGTH, TERRAIN_CHUNK_SIZE}, terrain_generation::WaitForTerrainGeneration};

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
            horizontal: 20,
            vertical: 8,
        }
    }
}

pub fn update_chunk_entities(
    mut commands: Commands,
    mut chunk_entities: ResMut<ChunkEntities>,
    render_distance_params: Res<RenderDistanceParams>,
) {
    let player_chunk = render_distance_params.player_chunk;
    let h = render_distance_params.horizontal;
    let v = render_distance_params.vertical;
    let mut to_create = Vec::new();
    for (x, y, z) in iproduct!(-h..=h, -v..=v, -h..=h) {
        let chunk_pos = player_chunk + IVec3::new(x, y, z);
        if x * x + z * z > h * h {
            continue;
        }
        if !chunk_entities.entities.contains_key(&chunk_pos) {
            to_create.push(chunk_pos);
        }
    }
    let mut to_remove = Vec::new();
    // 1チャンク移動するだけで削除と追加を繰り返すのは非効率なので,
    // 削除対象は余裕をもって判定する
    for chunk_pos in chunk_entities.entities.keys() {
        let offset = *chunk_pos - player_chunk;
        if offset.y.abs() > v + 2 || offset.x * offset.x + offset.z * offset.z > (h + 2) * (h + 2) {
            to_remove.push(*chunk_pos);
        }
    }
    info!("Creating {} chunks, removing {} chunks", to_create.len(), to_remove.len());
    for chunk_pos in to_create {
        let entity = commands.spawn((
            TerrainChunk {
                position: chunk_pos,
            },
            WaitForTerrainGeneration,
            Transform::from_translation(chunk_pos.as_vec3() * TERRAIN_CHUNK_LENGTH),
            InheritedVisibility::default(),
        )).id();
        chunk_entities.entities.insert(chunk_pos, entity);
    }
    for chunk_pos in to_remove {
        if let Some(entity) = chunk_entities.entities.remove(&chunk_pos) {
            commands.entity(entity).despawn();
        }
    }
}