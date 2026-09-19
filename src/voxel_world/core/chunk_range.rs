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
    let unload_distance = ((params.horizontal as f32 * horizontal_factor).ceil() as i32)
        .max(params.horizontal + ACTIVE_RANGE_MARGIN_CHUNKS);
    let unload_distance_sq = unload_distance * unload_distance;
    let vertical_unload_distance = ((params.vertical as f32 * horizontal_factor).ceil() as i32)
        .max(params.vertical + ACTIVE_RANGE_MARGIN_CHUNKS);

    let offset = chunk_pos - params.player_chunk;
    offset.x * offset.x + offset.z * offset.z > unload_distance_sq
        || offset.y.abs() > vertical_unload_distance
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> RenderDistanceParams {
        RenderDistanceParams {
            player_chunk: IVec3::ZERO,
            horizontal: 16,
            vertical: 4,
        }
    }

    #[test]
    fn unload_distance_applies_to_horizontal_and_vertical_axes() {
        let params = params();

        assert!(!should_unload_chunk(IVec3::new(24, 6, 0), &params));
        assert!(should_unload_chunk(IVec3::new(25, 0, 0), &params));
        assert!(should_unload_chunk(IVec3::new(0, 7, 0), &params));
    }

    #[test]
    fn unload_boundary_never_overlaps_the_active_margin() {
        let params = RenderDistanceParams {
            player_chunk: IVec3::ZERO,
            horizontal: 0,
            vertical: 0,
        };

        assert!(!should_unload_chunk(
            IVec3::new(ACTIVE_RANGE_MARGIN_CHUNKS, 0, 0),
            &params,
        ));
        assert!(!should_unload_chunk(
            IVec3::new(0, ACTIVE_RANGE_MARGIN_CHUNKS, 0),
            &params,
        ));
    }
}
