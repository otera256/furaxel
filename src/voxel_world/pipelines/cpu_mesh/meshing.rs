use bevy::prelude::*;
use bevy::tasks::futures::check_ready;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use crate::voxel_world::{
    core::{ChunkContentRevision, ChunkEntities, ChunkMeshDirty, TerrainChunk},
    storage::ChunkMap,
};
use super::material::{MaterialRepository, VoxelMaterialHandle};

// メッシュが作成中または既に作成されたチャンクに付与されるコンポーネント
#[derive(Component)]
pub struct MeshQueued;

#[derive(Component)]
pub struct NeedImmediateMeshUpdate;

#[derive(Component)]
pub struct ComputingMesh {
    task: Task<Vec<(VoxelMaterialHandle, Mesh)>>,
    input: MeshInputStamp,
}

/// The exact chunk-data versions used to produce a mesh.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshInputStamp {
    pub content_revision: u64,
    pub neighbor_content_revisions: [u64; 6],
}

/// Describes the currently committed mesh. It is intentionally separate from
/// the in-flight job so an old mesh can remain visible while a replacement is
/// being computed.
#[derive(Component, Debug, Clone)]
pub struct MeshArtifact {
    pub built_from: MeshInputStamp,
}

const MAX_MESH_TASKS_IN_FLIGHT: usize = 16;

const FACE_NEIGHBORS: [IVec3; 6] = [
    IVec3::NEG_X,
    IVec3::X,
    IVec3::NEG_Y,
    IVec3::Y,
    IVec3::NEG_Z,
    IVec3::Z,
];

fn mesh_input_stamp_from_revisions(
    chunk_pos: IVec3,
    mut revision_at: impl FnMut(IVec3) -> Option<u64>,
) -> Option<MeshInputStamp> {
    let content_revision = revision_at(chunk_pos)?;
    let mut neighbor_content_revisions = [0; 6];

    for (index, offset) in FACE_NEIGHBORS.into_iter().enumerate() {
        neighbor_content_revisions[index] = revision_at(chunk_pos + offset)?;
    }

    Some(MeshInputStamp {
        content_revision,
        neighbor_content_revisions,
    })
}

pub(super) fn current_mesh_input_stamp(
    chunk_pos: IVec3,
    chunk_entities: &ChunkEntities,
    revisions: &Query<&ChunkContentRevision>,
) -> Option<MeshInputStamp> {
    mesh_input_stamp_from_revisions(chunk_pos, |position| {
        let entity = *chunk_entities.entities.get(&position)?;
        revisions.get(entity).ok().map(|revision| revision.0)
    })
}

fn should_commit_mesh(computed: &MeshInputStamp, current: Option<&MeshInputStamp>) -> bool {
    current.is_some_and(|current| computed == current)
}

pub fn queue_mesh_tasks(
    mut commands: Commands,
    chunk_map: Res<ChunkMap>,
    material_repo: Res<MaterialRepository>,
    chunk_entities: Res<ChunkEntities>,
    revisions: Query<&ChunkContentRevision>,
    computing: Query<(), With<ComputingMesh>>,
    chunks: Query<(Entity, &TerrainChunk), (With<ChunkMeshDirty>, Without<ComputingMesh>, Without<NeedImmediateMeshUpdate>)>,
) {
    let thread_pool = AsyncComputeTaskPool::get();
    let available = MAX_MESH_TASKS_IN_FLIGHT.saturating_sub(computing.iter().count());

    for (entity, chunk) in chunks.iter().take(available) {
        let Some(input) = current_mesh_input_stamp(chunk.position, &chunk_entities, &revisions) else {
            continue;
        };

        if let Some(padded_chunk) = chunk_map.get_padded_chunk_vec(&chunk.position) {
            let material_repo = material_repo.clone();
            let task = thread_pool.spawn(async move {
                material_repo.create_mesh(padded_chunk)
            });
            commands.entity(entity)
                .remove::<ChunkMeshDirty>()
                .insert(ComputingMesh { task, input })
                .insert(MeshQueued);
        }
    }
}

pub fn handle_mesh_tasks(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    chunk_entities: Res<ChunkEntities>,
    revisions: Query<&ChunkContentRevision>,
    mut tasks: Query<(Entity, &TerrainChunk, &mut ComputingMesh, Option<&Children>)>,
) {
    for (entity, chunk, mut task, children) in &mut tasks {
        if let Some(generated_meshes) = check_ready(&mut task.task) {
            let current_input = current_mesh_input_stamp(
                chunk.position,
                &chunk_entities,
                &revisions,
            );

            if !should_commit_mesh(&task.input, current_input.as_ref()) {
                commands.entity(entity)
                    .remove::<ComputingMesh>()
                    .insert(ChunkMeshDirty);
                continue;
            }

            let committed_input = task.input.clone();
            commands.entity(entity).remove::<ComputingMesh>();
            
            // Despawn old meshes
            if let Some(children) = children {
                for child in children.iter() {
                    commands.entity(child).despawn();
                }
            }
            
            // Spawn new meshes
            commands.entity(entity).with_children(|parent| {
                for (material, mesh) in generated_meshes {
                    material.spawn(parent, meshes.add(mesh));
                }
            });
            commands.entity(entity).insert(MeshArtifact {
                built_from: committed_input,
            });
        }
    }
}

pub fn immediate_mesh_update(
    mut commands: Commands,
    chunk_map: Res<ChunkMap>,
    material_repo: Res<MaterialRepository>,
    mut meshes: ResMut<Assets<Mesh>>,
    chunk_entities: Res<ChunkEntities>,
    revisions: Query<&ChunkContentRevision>,
    chunks: Query<(Entity, &TerrainChunk, Option<&Children>), With<NeedImmediateMeshUpdate>>,
) {
    for (entity, chunk, children) in chunks.iter() {
        let Some(input) = current_mesh_input_stamp(chunk.position, &chunk_entities, &revisions) else {
            continue;
        };

        if let Some(padded_chunk) = chunk_map.get_padded_chunk_vec(&chunk.position) {
            let generated_meshes = material_repo.create_mesh(padded_chunk);
            
            // Despawn old meshes
            if let Some(children) = children {
                for child in children.iter() {
                    commands.entity(child).despawn();
                }
            }

            commands.entity(entity).with_children(|parent| {
                for (material, mesh) in generated_meshes {
                    material.spawn(parent, meshes.add(mesh));
                }
            });
            commands.entity(entity).insert(MeshArtifact { built_from: input });
        }
        commands.entity(entity)
            .remove::<NeedImmediateMeshUpdate>()
            .remove::<ChunkMeshDirty>()
            .insert(MeshQueued);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::platform::collections::HashMap;

    fn complete_revisions(center: IVec3, revision: u64) -> HashMap<IVec3, u64> {
        let mut revisions = HashMap::new();
        revisions.insert(center, revision);
        for offset in FACE_NEIGHBORS {
            revisions.insert(center + offset, revision);
        }
        revisions
    }

    #[test]
    fn stamp_requires_all_face_neighbors() {
        let center = IVec3::ZERO;
        let mut revisions = complete_revisions(center, 1);
        revisions.remove(&IVec3::X);

        let stamp = mesh_input_stamp_from_revisions(center, |pos| revisions.get(&pos).copied());

        assert!(stamp.is_none());
    }

    #[test]
    fn changing_a_neighbor_revision_invalidates_the_stamp() {
        let center = IVec3::ZERO;
        let mut revisions = complete_revisions(center, 1);
        let before = mesh_input_stamp_from_revisions(center, |pos| revisions.get(&pos).copied())
            .expect("complete neighborhood should produce a stamp");

        revisions.insert(IVec3::X, 2);
        let after = mesh_input_stamp_from_revisions(center, |pos| revisions.get(&pos).copied())
            .expect("complete neighborhood should produce a stamp");

        assert_ne!(before, after);
        assert_eq!(after.neighbor_content_revisions[1], 2);
        assert!(!should_commit_mesh(&before, Some(&after)));
        assert!(should_commit_mesh(&after, Some(&after)));
        assert!(!should_commit_mesh(&after, None));
    }
}
