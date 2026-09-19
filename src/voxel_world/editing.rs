use bevy::prelude::*;

pub use crate::voxel_world::edit_store::{VoxelEditPersistence, VoxelEditStore};

use crate::voxel_world::{
    core::{ChunkContentRevision, ChunkEntities, ChunkMeshDirty, Voxel},
    edit_store::{load_voxel_edits, save_dirty_voxel_edits},
    storage::{ChunkMap, VoxelMutationReport, VoxelWritePolicy},
};

pub struct VoxelEditingPlugin;

impl Plugin for VoxelEditingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VoxelEditStore>()
            .init_resource::<VoxelEditPersistence>()
            .add_message::<VoxelEditRequest>()
            .add_systems(Startup, load_voxel_edits)
            .add_systems(
                Update,
                (apply_requested_voxel_edits, save_dirty_voxel_edits).chain(),
            );
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
    mut edit_store: ResMut<VoxelEditStore>,
) {
    for request in requests.read() {
        // Record first: edits aimed at an unloaded chunk must not be lost.
        edit_store.record(&request.changes);
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
            .insert_resource(VoxelEditPersistence { path: None })
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
        assert_eq!(
            app.world()
                .resource::<VoxelEditStore>()
                .changes_for_chunk(IVec3::ZERO),
            vec![(IVec3::new(1, 1, 1), Voxel::STONE)],
        );
    }

    #[test]
    fn edit_to_an_unloaded_chunk_is_retained() {
        let mut app = App::new();
        app.add_plugins(VoxelEditingPlugin)
            .insert_resource(VoxelEditPersistence { path: None })
            .insert_resource(ChunkMap::default())
            .insert_resource(ChunkEntities::default());

        let position = IVec3::new(-65, 130, 7);
        app.world_mut()
            .write_message(VoxelEditRequest::single(position, Voxel::STONE));
        app.update();

        assert_eq!(app.world().resource::<ChunkMap>().get_at(position), None);
        assert_eq!(
            app.world()
                .resource::<VoxelEditStore>()
                .changes_for_chunk(IVec3::new(-2, 2, 0)),
            vec![(position, Voxel::STONE)],
        );
    }
}
