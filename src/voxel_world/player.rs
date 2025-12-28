use bevy::prelude::*;

use crate::voxel_world::{chunking::RenderDistanceParams, terrain_chunk::TERRAIN_CHUNK_LENGTH};

// Playerを識別するためのマーカーコンポーネント
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component)]
pub struct Player;

pub fn update_player_chunk(
    player_transform: Single<&Transform, With<Player>>,
    mut render_distance_params: ResMut<RenderDistanceParams>,
) {
    let player_pos = player_transform.translation;
    let player_chunk = (player_pos / TERRAIN_CHUNK_LENGTH).floor().as_ivec3();
    // run_if(rsource_changed::<T>) で真の変更のみを検知するために、
    // 明示的に変更があった場合のみ更新する
    if render_distance_params.player_chunk != player_chunk {
        render_distance_params.player_chunk = player_chunk;
    }
}