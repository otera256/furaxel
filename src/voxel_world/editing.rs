use bevy::prelude::*;

use crate::voxel_world::{
    core::{ChunkContentRevision, ChunkEntities, ChunkMeshDirty, Voxel},
    storage::{ChunkMap, VoxelMutationReport, VoxelWritePolicy},
};

pub struct VoxelEditingPlugin;

impl Plugin for VoxelEditingPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<VoxelEditRequest>()
            .add_systems(Update, apply_requested_voxel_edits);
    }
}

/// Public entry point for gameplay voxel edits.
///
/// Sending a request guarantees that successful changes advance the owning
/// chunk revision and invalidate every loaded mesh that observes the changed
/// voxel.
#[derive(Message, Debug, Clone)]
pub struct VoxelEditRequest {
    pub changes: Vec<(IVec3, Voxel)>,
}

impl VoxelEditRequest {
    #[allow(dead_code)]
    pub fn single(position: IVec3, voxel: Voxel) -> Self {
        Self {
            changes: vec![(position, voxel)],
        }
    }

    #[allow(dead_code)]
    pub fn batch(changes: Vec<(IVec3, Voxel)>) -> Self {
        Self { changes }
    }
}

pub(crate) fn apply_voxel_changes(
    commands: &mut Commands,
    chunk_map: &mut ChunkMap,
    chunk_entities: &ChunkEntities,
    revisions: &mut Query<&mut ChunkContentRevision>,
    changes: Vec<(IVec3, Voxel)>,
    policy: VoxelWritePolicy,
) -> VoxelMutationReport {
    let report = chunk_map.apply_voxel_changes(changes, policy);

    for chunk_pos in &report.changed_chunks {
        if let Some(entity) = chunk_entities.entities.get(chunk_pos)
            && let Ok(mut revision) = revisions.get_mut(*entity)
        {
            revision.advance();
        }
    }

    for chunk_pos in &report.mesh_dirty_chunks {
        if let Some(entity) = chunk_entities.entities.get(chunk_pos) {
            commands.entity(*entity).insert(ChunkMeshDirty);
        }
    }

    report
}

fn apply_requested_voxel_edits(
    mut commands: Commands,
    mut requests: MessageReader<VoxelEditRequest>,
    mut chunk_map: ResMut<ChunkMap>,
    chunk_entities: Res<ChunkEntities>,
    mut revisions: Query<&mut ChunkContentRevision>,
) {
    for request in requests.read() {
        apply_voxel_changes(
            &mut commands,
            &mut chunk_map,
            &chunk_entities,
            &mut revisions,
            request.changes.clone(),
            VoxelWritePolicy::Always,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voxel_world::core::TerrainChunkData;

    #[test]
    fn edit_request_advances_revision_and_marks_mesh_dirty() {
        let mut app = App::new();
        app.add_plugins(VoxelEditingPlugin)
            .insert_resource(ChunkMap::default())
            .insert_resource(ChunkEntities::default());

        let entity = app.world_mut().spawn(ChunkContentRevision::default()).id();
        app.world_mut()
            .resource_mut::<ChunkEntities>()
            .entities
            .insert(IVec3::ZERO, entity);
        app.world_mut()
            .resource_mut::<ChunkMap>()
            .insert(TerrainChunkData::new_empty(IVec3::ZERO));
        app.world_mut().write_message(VoxelEditRequest::single(
            IVec3::new(1, 1, 1),
            Voxel::STONE,
        ));

        app.update();

        assert_eq!(app.world().get::<ChunkContentRevision>(entity).unwrap().0, 1);
        assert!(app.world().get::<ChunkMeshDirty>(entity).is_some());
        assert_eq!(
            app.world().resource::<ChunkMap>().get_at(IVec3::new(1, 1, 1)),
            Some(Voxel::STONE),
        );
    }
}
