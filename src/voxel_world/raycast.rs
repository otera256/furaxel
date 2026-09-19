use bevy::math::{IVec3, Vec3};

use crate::voxel_world::{core::Voxel, storage::ChunkMap};

/// A voxel intersected by a camera ray and the last empty cell before it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VoxelRayHit {
    pub position: IVec3,
    pub previous_empty_position: Option<IVec3>,
    pub voxel: Voxel,
    pub distance: f32,
}

/// Traverses voxel cells with a 3D DDA. Returning `None` from `sample` means
/// that the world data is unavailable; the ray stops instead of editing an
/// unloaded chunk as though it were empty space.
pub fn raycast_voxels(
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
    mut sample: impl FnMut(IVec3) -> Option<Voxel>,
) -> Option<VoxelRayHit> {
    if max_distance < 0.0 || !max_distance.is_finite() {
        return None;
    }
    let direction = direction.normalize_or_zero();
    if direction == Vec3::ZERO {
        return None;
    }

    // Move an infinitesimal amount into the ray to make exact grid-boundary
    // origins choose the cell in the direction of travel, including negatives.
    let origin = origin + direction * 1e-5;
    let mut cell = origin.floor().as_ivec3();
    let step = IVec3::new(sign(direction.x), sign(direction.y), sign(direction.z));
    let t_delta = Vec3::new(
        reciprocal_abs(direction.x),
        reciprocal_abs(direction.y),
        reciprocal_abs(direction.z),
    );
    let mut next_boundary = Vec3::new(
        next_boundary_distance(origin.x, cell.x, direction.x),
        next_boundary_distance(origin.y, cell.y, direction.y),
        next_boundary_distance(origin.z, cell.z, direction.z),
    );
    let mut distance = 0.0;
    let mut previous_empty_position = None;

    loop {
        let voxel = sample(cell)?;
        if voxel != Voxel::EMPTY {
            return Some(VoxelRayHit {
                position: cell,
                previous_empty_position,
                voxel,
                distance,
            });
        }

        let next_distance = next_boundary.min_element();
        if !next_distance.is_finite() || next_distance > max_distance {
            return None;
        }
        previous_empty_position = Some(cell);
        distance = next_distance;

        // Advance every tied axis. This keeps corner crossings deterministic
        // and prevents visiting the same cell twice.
        let epsilon = 1e-6;
        if (next_boundary.x - next_distance).abs() <= epsilon {
            cell.x += step.x;
            next_boundary.x += t_delta.x;
        }
        if (next_boundary.y - next_distance).abs() <= epsilon {
            cell.y += step.y;
            next_boundary.y += t_delta.y;
        }
        if (next_boundary.z - next_distance).abs() <= epsilon {
            cell.z += step.z;
            next_boundary.z += t_delta.z;
        }
    }
}

pub fn raycast_chunk_map(
    chunk_map: &ChunkMap,
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
) -> Option<VoxelRayHit> {
    raycast_voxels(origin, direction, max_distance, |position| {
        chunk_map.get_at(position)
    })
}

fn sign(value: f32) -> i32 {
    if value > 0.0 {
        1
    } else if value < 0.0 {
        -1
    } else {
        0
    }
}

fn reciprocal_abs(value: f32) -> f32 {
    if value == 0.0 {
        f32::INFINITY
    } else {
        value.abs().recip()
    }
}

fn next_boundary_distance(origin: f32, cell: i32, direction: f32) -> f32 {
    if direction > 0.0 {
        (cell as f32 + 1.0 - origin) / direction
    } else if direction < 0.0 {
        (origin - cell as f32) / -direction
    } else {
        f32::INFINITY
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::platform::collections::HashMap;

    fn map(cells: &[(IVec3, Voxel)]) -> HashMap<IVec3, Voxel> {
        cells.iter().copied().collect()
    }

    #[test]
    fn hits_first_solid_voxel_and_reports_previous_empty_cell() {
        let cells = map(&[(IVec3::new(3, 0, 0), Voxel::STONE)]);
        let hit = raycast_voxels(
            Vec3::new(0.5, 0.5, 0.5),
            Vec3::X,
            10.0,
            |position| Some(cells.get(&position).copied().unwrap_or(Voxel::EMPTY)),
        )
        .unwrap();

        assert_eq!(hit.position, IVec3::new(3, 0, 0));
        assert_eq!(hit.previous_empty_position, Some(IVec3::new(2, 0, 0)));
        assert_eq!(hit.voxel, Voxel::STONE);
        assert!((hit.distance - 2.5).abs() < 1e-4);
    }

    #[test]
    fn supports_negative_coordinates_and_negative_direction() {
        let cells = map(&[(IVec3::new(-3, 0, 0), Voxel::DIRT)]);
        let hit = raycast_voxels(
            Vec3::new(-0.5, 0.5, 0.5),
            -Vec3::X,
            10.0,
            |position| Some(cells.get(&position).copied().unwrap_or(Voxel::EMPTY)),
        )
        .unwrap();

        assert_eq!(hit.position, IVec3::new(-3, 0, 0));
        assert_eq!(hit.previous_empty_position, Some(IVec3::new(-2, 0, 0)));
    }

    #[test]
    fn starting_inside_solid_has_no_placement_cell() {
        let hit = raycast_voxels(
            Vec3::new(1.5, 1.5, 1.5),
            Vec3::Z,
            10.0,
            |position| {
                Some(if position == IVec3::ONE {
                    Voxel::STONE
                } else {
                    Voxel::EMPTY
                })
            },
        )
        .unwrap();

        assert_eq!(hit.position, IVec3::ONE);
        assert_eq!(hit.previous_empty_position, None);
        assert_eq!(hit.distance, 0.0);
    }

    #[test]
    fn respects_max_distance_and_unloaded_cells() {
        let cells = map(&[(IVec3::new(3, 0, 0), Voxel::STONE)]);
        assert!(raycast_voxels(
            Vec3::new(0.5, 0.5, 0.5),
            Vec3::X,
            2.0,
            |position| Some(cells.get(&position).copied().unwrap_or(Voxel::EMPTY)),
        )
        .is_none());

        assert!(raycast_voxels(
            Vec3::new(0.5, 0.5, 0.5),
            Vec3::X,
            10.0,
            |position| cells.get(&position).copied(),
        )
        .is_none());
    }

    #[test]
    fn zero_direction_and_invalid_distance_are_misses() {
        assert!(raycast_voxels(Vec3::ZERO, Vec3::ZERO, 1.0, |_| Some(Voxel::STONE)).is_none());
        assert!(raycast_voxels(Vec3::ZERO, Vec3::X, -1.0, |_| Some(Voxel::STONE)).is_none());
        assert!(raycast_voxels(Vec3::ZERO, Vec3::X, f32::NAN, |_| Some(Voxel::STONE)).is_none());
    }
}
