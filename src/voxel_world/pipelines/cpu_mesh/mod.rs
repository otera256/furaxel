pub mod meshing;
pub mod material;
pub mod water;

use bevy::prelude::*;
use itertools::iproduct;

use crate::voxel_world::{
    core::{ChunkEntities, ChunkGeneratedEvent},
    pipelines::{
        cpu_noise::storage::TerrainGenerationStorage,
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
                queue_mesh_tasks,
                handle_mesh_tasks,
                immediate_mesh_update,
                trigger_mesh_update,
            ));
    }
}

// 他のチャンクの生成完了イベントを受け取り、メッシュ更新が必要なチャンクにNeedMeshUpdateコンポーネントを追加する
fn trigger_mesh_update(
    mut commands: Commands,
    mut events: MessageReader<ChunkGeneratedEvent>,
    storage: Res<TerrainGenerationStorage>,
    chunk_entities: Res<ChunkEntities>,
    mesh_queued_query: Query<&MeshQueued>,
) {
    for event in events.read() {
        let chunk_pos = event.0;
        
        let mut candidates = Vec::new();
        candidates.push(chunk_pos);
        for (dx, dy, dz) in iproduct!(-1..=1, -1..=1, -1..=1) {
            if dx == 0 && dy == 0 && dz == 0 { continue; }
            candidates.push(chunk_pos + IVec3::new(dx, dy, dz));
        }

        for pos in candidates {
            if storage.fully_generated.contains(&pos) {
                if let Some(entity) = chunk_entities.entities.get(&pos) {
                    let has_mesh = mesh_queued_query.get(*entity).is_ok();

                    let all_neighbors_ready = iproduct!(-1..=1, -1..=1, -1..=1)
                        .all(|(dx, dy, dz)| {
                            let neighbor_pos = pos + IVec3::new(dx, dy, dz);
                            storage.fully_generated.contains(&neighbor_pos)
                        });

                    if !has_mesh && all_neighbors_ready {
                        let entity = *entity;
                        commands.queue(move |world: &mut World| {
                            if let Ok(mut entity_world) = world.get_entity_mut(entity) {
                                entity_world.insert(NeedMeshUpdate);
                            }
                        });
                    }
                }
            }
        }
    }
}
