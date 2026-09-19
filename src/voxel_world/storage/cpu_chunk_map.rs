use std::vec;
use std::collections::HashSet;

use bevy::{ecs::resource::Resource, math::{IVec3, UVec3}, platform::collections::HashMap};
use block_mesh::ndshape::ConstShape;
use itertools::iproduct;

use crate::voxel_world::core::{chunk::Chunk, terrain_chunk::{PaddedTerrainChunkShape, TerrainChunkData}, coordinates::TERRAIN_CHUNK_SIZE, voxel::Voxel};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoxelWritePolicy {
    Always,
    ReplaceSoft,
}

#[derive(Debug, Default)]
pub struct VoxelMutationReport {
    pub changed_chunks: HashSet<IVec3>,
    pub mesh_dirty_chunks: HashSet<IVec3>,
    pub skipped_positions: Vec<IVec3>,
}

#[derive(Debug, Resource, Default)]
pub struct ChunkMap {
    chunks: HashMap<IVec3, TerrainChunkData>,
}

#[allow(dead_code)]
impl ChunkMap {
    pub fn new() -> Self {
        Self {
            chunks: HashMap::new(),
        }
    }

    pub fn insert(&mut self, chunk: TerrainChunkData) {
        self.chunks.insert(chunk.position, chunk);
    }

    pub fn positions(&self) -> impl Iterator<Item = &IVec3> {
        self.chunks.keys()
    }

    pub fn remove(&mut self, position: &IVec3) -> Option<TerrainChunkData> {
        self.chunks.remove(position)
    }

    pub fn get(&self, position: &IVec3) -> Option<&TerrainChunkData> {
        self.chunks.get(position)
    }
    pub fn get_slice(&self, position: &IVec3) -> Option<&[Voxel]> {
        let chunk = self.chunks.get(position)?;
        Some(chunk.chunk.as_slice())
    }
    // meshingする際に使用。隣接する6チャンクの1層分を取り込んで取得する
    // positionのチャンクが存在しないときはNoneを返す
    // 隣接するチャンクが存在しないときはEMPTY_VOXELで埋める
    pub fn get_padded_chunk_vec(&self, position: &IVec3) -> Option<Chunk<PaddedTerrainChunkShape>> {
        let center_chunk = self.chunks.get(position)?;
        let mut padded_voxels = vec![Voxel::EMPTY; PaddedTerrainChunkShape::USIZE];
        // 中心チャンクをコピー
        for (x, y, z) in iproduct!(0..TERRAIN_CHUNK_SIZE, 0..TERRAIN_CHUNK_SIZE, 0..TERRAIN_CHUNK_SIZE) {
            let voxel = center_chunk.get_local_at(UVec3::new(x, y, z));
            let index = PaddedTerrainChunkShape::linearize([x + 1, y + 1, z + 1]) as usize;
            padded_voxels[index] = voxel;
        }
        // 6方向の隣接チャンクをコピー
        // -X (West) 方向
        if let Some(west_chunk) = self.chunks.get(&( *position + IVec3::new(-1, 0, 0))) {
            for (y, z) in iproduct!(0..TERRAIN_CHUNK_SIZE, 0..TERRAIN_CHUNK_SIZE) {
                let voxel = west_chunk.get_local_at(UVec3::new(TERRAIN_CHUNK_SIZE - 1, y, z));
                let index = PaddedTerrainChunkShape::linearize([0, y + 1, z + 1]) as usize;
                padded_voxels[index] = voxel;
            }
        }
        // +X (East) 方向
        if let Some(east_chunk) = self.chunks.get(&( *position + IVec3::new(1, 0, 0))) {
            for (y, z) in iproduct!(0..TERRAIN_CHUNK_SIZE, 0..TERRAIN_CHUNK_SIZE) {
                let voxel = east_chunk.get_local_at(UVec3::new(0, y, z));
                let index = PaddedTerrainChunkShape::linearize([TERRAIN_CHUNK_SIZE + 1, y + 1, z + 1]) as usize;
                padded_voxels[index] = voxel;
            }
        }
        // -Y (Down) 方向
        if let Some(down_chunk) = self.chunks.get(&( *position + IVec3::new(0, -1, 0))) {
            for (x, z) in iproduct!(0..TERRAIN_CHUNK_SIZE, 0..TERRAIN_CHUNK_SIZE) {
                let voxel = down_chunk.get_local_at(UVec3::new(x, TERRAIN_CHUNK_SIZE - 1, z));
                let index = PaddedTerrainChunkShape::linearize([x + 1, 0, z + 1]) as usize;
                padded_voxels[index] = voxel;
            }
        }
        // +Y (Up) 方向
        if let Some(up_chunk) = self.chunks.get(&( *position + IVec3::new(0, 1, 0))) {
            for (x, z) in iproduct!(0..TERRAIN_CHUNK_SIZE, 0..TERRAIN_CHUNK_SIZE) {
                let voxel = up_chunk.get_local_at(UVec3::new(x, 0, z));
                let index = PaddedTerrainChunkShape::linearize([x + 1, TERRAIN_CHUNK_SIZE + 1, z + 1]) as usize;
                padded_voxels[index] = voxel;
            }
        }
        // -Z (North) 方向
        if let Some(north_chunk) = self.chunks.get(&( *position + IVec3::new(0, 0, -1))) {
            for (x, y) in iproduct!(0..TERRAIN_CHUNK_SIZE, 0..TERRAIN_CHUNK_SIZE) {
                let voxel = north_chunk.get_local_at(UVec3::new(x, y, TERRAIN_CHUNK_SIZE - 1));
                let index = PaddedTerrainChunkShape::linearize([x + 1, y + 1, 0]) as usize;
                padded_voxels[index] = voxel;
            }
        }
        // +Z (South) 方向
        if let Some(south_chunk) = self.chunks.get(&( *position + IVec3::new(0, 0, 1))) {
            for (x, y) in iproduct!(0..TERRAIN_CHUNK_SIZE, 0..TERRAIN_CHUNK_SIZE) {
                let voxel = south_chunk.get_local_at(UVec3::new(x, y, 0));
                let index = PaddedTerrainChunkShape::linearize([x + 1, y + 1, TERRAIN_CHUNK_SIZE + 1]) as usize;
                padded_voxels[index] = voxel;
            }
        }
        Some(Chunk {
            voxels: padded_voxels.into_boxed_slice(),
            shape: PaddedTerrainChunkShape {},
        })
    }

    pub(crate) fn apply_voxel_changes(
        &mut self,
        changes: Vec<(IVec3, Voxel)>,
        policy: VoxelWritePolicy,
    ) -> VoxelMutationReport {
        let mut report = VoxelMutationReport::default();
        let mut changes_by_chunk: HashMap<IVec3, Vec<(IVec3, Voxel)>> = HashMap::new();
        
        for (world_pos, voxel) in changes {
            let chunk_pos = world_pos.div_euclid(IVec3::splat(TERRAIN_CHUNK_SIZE as i32));
            changes_by_chunk.entry(chunk_pos).or_default().push((world_pos, voxel));
        }

        for (chunk_pos, chunk_changes) in changes_by_chunk {
            let Some(chunk) = self.chunks.get_mut(&chunk_pos) else {
                report.skipped_positions.extend(chunk_changes.into_iter().map(|(pos, _)| pos));
                continue;
            };

            for (world_pos, voxel) in chunk_changes {
                let local_pos = (world_pos.rem_euclid(IVec3::splat(TERRAIN_CHUNK_SIZE as i32))).as_uvec3();
                let target_voxel = chunk.get_local_at_mut(local_pos);
                let can_replace = match policy {
                    VoxelWritePolicy::Always => true,
                    VoxelWritePolicy::ReplaceSoft => {
                        target_voxel.id == Voxel::EMPTY.id
                            || target_voxel.id == Voxel::WATER.id
                            || target_voxel.id == Voxel::SNOW.id
                    }
                };

                if !can_replace {
                    report.skipped_positions.push(world_pos);
                    continue;
                }

                if *target_voxel == voxel {
                    continue;
                }

                *target_voxel = voxel;
                report.changed_chunks.insert(chunk_pos);
                report.mesh_dirty_chunks.insert(chunk_pos);

                let max = TERRAIN_CHUNK_SIZE - 1;
                if local_pos.x == 0 { report.mesh_dirty_chunks.insert(chunk_pos + IVec3::NEG_X); }
                if local_pos.x == max { report.mesh_dirty_chunks.insert(chunk_pos + IVec3::X); }
                if local_pos.y == 0 { report.mesh_dirty_chunks.insert(chunk_pos + IVec3::NEG_Y); }
                if local_pos.y == max { report.mesh_dirty_chunks.insert(chunk_pos + IVec3::Y); }
                if local_pos.z == 0 { report.mesh_dirty_chunks.insert(chunk_pos + IVec3::NEG_Z); }
                if local_pos.z == max { report.mesh_dirty_chunks.insert(chunk_pos + IVec3::Z); }
            }
        }

        report
    }

    pub fn get_at(&self, world_pos: IVec3) -> Option<Voxel> {
        let chunk_pos = world_pos.div_euclid(IVec3::splat(TERRAIN_CHUNK_SIZE as i32));
        let chunk = self.chunks.get(&chunk_pos)?;
        Some(chunk.get_at(world_pos))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map_with_empty_chunk(position: IVec3) -> ChunkMap {
        let mut map = ChunkMap::new();
        map.insert(TerrainChunkData::new_empty(position));
        map
    }

    #[test]
    fn interior_edit_only_invalidates_its_own_chunk() {
        let mut map = map_with_empty_chunk(IVec3::ZERO);
        let position = IVec3::new(1, 2, 3);

        let report = map.apply_voxel_changes(
            vec![(position, Voxel::STONE)],
            VoxelWritePolicy::Always,
        );

        assert_eq!(map.get_at(position), Some(Voxel::STONE));
        assert_eq!(report.changed_chunks, HashSet::from([IVec3::ZERO]));
        assert_eq!(report.mesh_dirty_chunks, HashSet::from([IVec3::ZERO]));
    }

    #[test]
    fn negative_boundary_edit_invalidates_the_touching_neighbor() {
        let chunk_pos = IVec3::new(-1, 0, 0);
        let mut map = map_with_empty_chunk(chunk_pos);
        let position = IVec3::new(-1, 1, 1);

        let report = map.apply_voxel_changes(
            vec![(position, Voxel::STONE)],
            VoxelWritePolicy::Always,
        );

        assert!(report.mesh_dirty_chunks.contains(&chunk_pos));
        assert!(report.mesh_dirty_chunks.contains(&IVec3::ZERO));
        assert_eq!(report.mesh_dirty_chunks.len(), 2);
    }

    #[test]
    fn replace_soft_does_not_overwrite_solid_voxels() {
        let mut map = map_with_empty_chunk(IVec3::ZERO);
        let position = IVec3::new(1, 1, 1);
        map.apply_voxel_changes(
            vec![(position, Voxel::STONE)],
            VoxelWritePolicy::Always,
        );

        let report = map.apply_voxel_changes(
            vec![(position, Voxel::FLOWER_RED)],
            VoxelWritePolicy::ReplaceSoft,
        );

        assert_eq!(map.get_at(position), Some(Voxel::STONE));
        assert!(report.changed_chunks.is_empty());
        assert_eq!(report.skipped_positions, vec![position]);
    }
}
