use bevy::math::IVec3;

use super::components::RenderDistanceParams;

pub const ACTIVE_RANGE_MARGIN_CHUNKS: i32 = 2;
pub const UNLOAD_DISTANCE_FACTOR: f32 = 1.5;

pub fn is_within_active_chunk_range(chunk_pos: IVec3, params: &RenderDistanceParams) -> bool {
    is_within_active_chunk_range_with_margin(chunk_pos, params, ACTIVE_RANGE_MARGIN_CHUNKS)
}

pub fn is_within_active_chunk_range_with_margin(
    chunk_pos: IVec3,
    params: &RenderDistanceParams,
    margin_chunks: i32,
) -> bool {
    let offset = chunk_pos - params.player_chunk;

    if offset.y.abs() > params.vertical + margin_chunks {
        return false;
    }

    let r = params.horizontal + margin_chunks;
    offset.x * offset.x + offset.z * offset.z <= r * r
}

pub fn should_unload_chunk(chunk_pos: IVec3, params: &RenderDistanceParams) -> bool {
    should_unload_chunk_with_factor(chunk_pos, params, UNLOAD_DISTANCE_FACTOR)
}

pub fn should_unload_chunk_with_factor(
    chunk_pos: IVec3,
    params: &RenderDistanceParams,
    horizontal_factor: f32,
) -> bool {
    let unload_distance = (params.horizontal as f32 * horizontal_factor) as i32;
    let unload_distance_sq = unload_distance * unload_distance;

    let offset = chunk_pos - params.player_chunk;
    offset.x * offset.x + offset.z * offset.z > unload_distance_sq
}
