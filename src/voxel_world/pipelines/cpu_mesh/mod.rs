pub mod meshing;
pub mod material;
pub mod water;

use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use std::time::Duration;

use crate::voxel_world::{
    core::{ChunkEntities, ChunkGeneratedEvent, ChunkGenerationComplete, ChunkMeshDirty, TerrainChunk},
    pipelines::{
        cpu_mesh::{material::*, meshing::*, water::WaterMaterial},
    }
};

#[derive(Default)]
pub struct CpuMeshRenderingPlugin;

impl Plugin for CpuMeshRenderingPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(MaterialPlugin::<WaterMaterial>::default())
            .insert_resource(MaterialRepository::default())
            .add_systems(Startup, material_setup)
            .add_systems(Update, (
                handle_mesh_tasks,
                immediate_mesh_update,
                trigger_mesh_update,
                reconcile_missing_meshes.run_if(on_timer(Duration::from_millis(500))),
                queue_mesh_tasks,
            ).chain());
    }
}

// 他のチャンクの生成完了イベントを受け取り、メッシュ更新が必要なチャンクをdirtyにする
fn trigger_mesh_update(
    mut commands: Commands,
    mut events: MessageReader<ChunkGeneratedEvent>,
    chunk_entities: Res<ChunkEntities>,
    generation_complete: Query<(), With<ChunkGenerationComplete>>,
    revisions: Query<&crate::voxel_world::core::ChunkContentRevision>,
    mesh_state_query: Query<(Option<&ComputingMesh>, Option<&MeshArtifact>)>,
) {
    for event in events.read() {
        let chunk_pos = event.0;
        
        let candidates = std::iter::once(chunk_pos)
            .chain(FACE_NEIGHBORS.into_iter().map(|offset| chunk_pos + offset));

        for pos in candidates {
            if let Some(entity) = chunk_entities.entities.get(&pos) {
                if generation_complete.get(*entity).is_ok() {
                    let all_neighbors_ready = face_neighbors_ready(pos, |neighbor_pos| {
                        chunk_entities.entities.get(&neighbor_pos)
                            .is_some_and(|entity| generation_complete.get(*entity).is_ok())
                    });

                    let current_input = current_mesh_input_stamp(
                        pos,
                        &chunk_entities,
                        &revisions,
                    );
                    let (is_computing, is_current) = mesh_state_query
                        .get(*entity)
                        .map(|(job, artifact)| {
                            (
                                job.is_some(),
                                artifact
                                    .zip(current_input.as_ref())
                                    .is_some_and(|(artifact, current)| {
                                        artifact.built_from == *current
                                    }),
                            )
                        })
                        .unwrap_or((false, false));

                    if !is_computing && !is_current && all_neighbors_ready {
                        let entity = *entity;
                        commands.queue(move |world: &mut World| {
                            if let Ok(mut entity_world) = world.get_entity_mut(entity) {
                                entity_world.insert(ChunkMeshDirty);
                            }
                        });
                    }
                }
            }
        }
    }
}

/// Event-driven invalidation is fast, but correctness must not depend on a
/// single frame's event delivery. This low-frequency pass repairs any complete
/// chunk that is missing a current mesh after rapid movement or task churn.
fn reconcile_missing_meshes(
    mut commands: Commands,
    chunk_entities: Res<ChunkEntities>,
    generation_complete: Query<(), With<ChunkGenerationComplete>>,
    revisions: Query<&crate::voxel_world::core::ChunkContentRevision>,
    chunks: Query<
        (Entity, &TerrainChunk, Option<&MeshArtifact>),
        (
            With<ChunkGenerationComplete>,
            Without<ComputingMesh>,
            Without<ChunkMeshDirty>,
            Without<NeedImmediateMeshUpdate>,
        ),
    >,
) {
    for (entity, chunk, artifact) in &chunks {
        let neighbors_ready = face_neighbors_ready(chunk.position, |neighbor_pos| {
            chunk_entities.entities.get(&neighbor_pos)
                .is_some_and(|entity| generation_complete.get(*entity).is_ok())
        });
        if !neighbors_ready {
            continue;
        }

        let current_input = current_mesh_input_stamp(
            chunk.position,
            &chunk_entities,
            &revisions,
        );
        let is_current = artifact
            .zip(current_input.as_ref())
            .is_some_and(|(artifact, current)| artifact.built_from == *current);

        if !is_current {
            commands.entity(entity).insert(ChunkMeshDirty);
        }
    }
}
