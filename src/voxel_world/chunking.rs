use bevy::{prelude::*, math::IVec3};
use itertools::iproduct;

use crate::voxel_world::{
    core::{
        chunk_range::{is_within_active_chunk_range, should_unload_chunk},
        coordinates::TERRAIN_CHUNK_LENGTH,
        ChunkEntities, RenderDistanceParams, TerrainChunk
    },
    storage::ChunkMap,
    pipelines::cpu_noise::WaitForTerrainGeneration
};

pub fn unload_distant_chunks(
    mut chunk_map: ResMut<ChunkMap>,
    render_distance_params: Res<RenderDistanceParams>,
) {
    let mut to_remove = Vec::new();
    for chunk_pos in chunk_map.chunks.keys() {
        if should_unload_chunk(*chunk_pos, &render_distance_params) {
            to_remove.push(*chunk_pos);
        }
    }

    if !to_remove.is_empty() {
        // info!("Unloading {} distant chunks", to_remove.len());
        for chunk_pos in to_remove {
            chunk_map.chunks.remove(&chunk_pos);
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
        if !is_within_active_chunk_range(*chunk_pos, &render_distance_params) {
            to_remove.push(*chunk_pos);
        }
    }
    // info!("Creating {} chunks, removing {} chunks", to_create.len(), to_remove.len());
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
            commands.entity(entity).despawn_children();
            commands.entity(entity).despawn();
        }
    }
}
