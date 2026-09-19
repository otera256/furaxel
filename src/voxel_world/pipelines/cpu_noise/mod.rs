pub mod biomes;
mod feature;
pub mod generation;

use bevy::{
    prelude::*,
    tasks::{futures::check_ready, AsyncComputeTaskPool, Task},
};
use itertools::Itertools;
use std::sync::Arc;
use bevy::platform::collections::HashMap;
use crate::voxel_world::{
    core::{chunk_range::is_within_active_chunk_range, terrain_chunk::TerrainChunkData, voxel::Voxel, ChunkBaseTerrainReady, ChunkContentRevision, ChunkEntities, ChunkGeneratedEvent, ChunkGenerationComplete, RenderDistanceParams, TerrainChunk},
    editing::apply_voxel_changes,
    storage::ChunkMap,
};
use self::biomes::BiomeRegistry;

#[derive(Default)]
pub struct CpuNoiseTerrainGenerationPlugin;

impl Plugin for CpuNoiseTerrainGenerationPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(TerrainColumns::default())
            .add_message::<ChunkGeneratedEvent>()
            .add_systems(Startup, setup_terrain_generation)
            .add_systems(Update, (
                ensure_terrain_columns,
                queue_altitude_tasks,
                handle_altitude_tasks,
                queue_base_terrain_tasks,
                handle_base_terrain_tasks,
                queue_feature_tasks,
                handle_feature_tasks,
                emit_chunk_generated_events,
            ))
            ;
    }
}

#[derive(Resource)]
pub struct WorldGenConfig {
    pub seed: u32,
    pub biome_registry: Arc<BiomeRegistry>,
}

fn setup_terrain_generation(mut commands: Commands) {
    let seed = 12345;
    commands.insert_resource(WorldGenConfig {
        seed,
        biome_registry: Arc::new(BiomeRegistry::new(seed)),
    });
}


#[derive(Component, Debug, Clone, Copy)]
pub struct WaitForTerrainGeneration;

#[derive(Component, Debug, Clone, Copy)]
struct TerrainColumn {
    position: IVec2,
}

#[derive(Component, Debug, Clone, Copy)]
struct TerrainColumnRef(Entity);

#[derive(Resource, Debug, Default)]
struct TerrainColumns {
    entities: HashMap<IVec2, Entity>,
}

#[derive(Component, Debug)]
struct WaitForAltitude;

#[derive(Component, Debug, Clone)]
struct AltitudeArtifact {
    altitude_map: Arc<[i32]>,
    biome_map: Arc<[u8]>,
}

#[derive(Component, Debug)]
struct ComputingAltitude(Task<AltitudeTaskResult>);

#[derive(Component)]
struct WaitForBaseTerrain;

#[derive(Component, Debug)]
struct ComputingBaseTerrain(Task<BaseTerrainTaskResult>);

#[derive(Component, Debug)]
struct WaitForNeighbors;

#[derive(Component, Debug)]
struct ComputingFeatures(Task<FeaturesTaskResult>);

#[derive(Debug)]
struct AltitudeTaskResult {
    altitude_map: Arc<[i32]>,
    biome_map: Arc<[u8]>,
}

#[derive(Debug)]
struct BaseTerrainTaskResult {
    chunk_data: TerrainChunkData,
}

#[derive(Debug)]
struct FeaturesTaskResult {
    changes: Vec<(IVec3, Voxel)>,
}

const MAX_COMPUTE_TERRAIN_TASKS_PER_FRAME: usize = 10;
const MAX_ALTITUDE_TASKS_IN_FLIGHT: usize = 16;
const MAX_BASE_TERRAIN_TASKS_IN_FLIGHT: usize = 32;
const MAX_FEATURE_TASKS_IN_FLIGHT: usize = 32;

fn ensure_terrain_columns(
    mut commands: Commands,
    chunks: Query<(Entity, &TerrainChunk), With<WaitForTerrainGeneration>>,
    mut columns: ResMut<TerrainColumns>,
) {
    for (chunk_entity, chunk) in &chunks {
        let column_pos = chunk.position.xz();
        let column_entity = *columns.entities.entry(column_pos).or_insert_with(|| {
            commands.spawn((TerrainColumn { position: column_pos }, WaitForAltitude)).id()
        });

        commands.entity(chunk_entity)
            .remove::<WaitForTerrainGeneration>()
            .insert((TerrainColumnRef(column_entity), WaitForBaseTerrain));
    }
}

fn queue_altitude_tasks(
    mut commands: Commands,
    render_distance_params: Res<RenderDistanceParams>,
    target_columns: Query<(Entity, &TerrainColumn), With<WaitForAltitude>>,
    computing: Query<(), With<ComputingAltitude>>,
    world_gen_config: Res<WorldGenConfig>,
) {
    let thread_pool = AsyncComputeTaskPool::get();
    let config = world_gen_config.biome_registry.clone();
    let seed = world_gen_config.seed;

    let available = MAX_ALTITUDE_TASKS_IN_FLIGHT.saturating_sub(computing.iter().count());
    let task_count = available.min(MAX_COMPUTE_TERRAIN_TASKS_PER_FRAME);

    for (entity, column) in target_columns
        .iter()
        .k_smallest_by_key(task_count, |(_, column)| {
            (column.position - render_distance_params.player_chunk.xz()).length_squared()
        })
    {
        let column_pos = column.position;
        let config = config.clone();
        let task = thread_pool.spawn(async move {
            let (altitude_map, biome_map) = generation::generate_altitude_map(seed, column_pos, &config);
            AltitudeTaskResult {
                altitude_map: altitude_map.into(),
                biome_map: biome_map.into(),
            }
        });
        commands.entity(entity)
            .remove::<WaitForAltitude>()
            .insert(ComputingAltitude(task));
    }
}

fn handle_altitude_tasks(
    mut commands: Commands,
    mut tasks: Query<(Entity, &mut ComputingAltitude)>,
) {
    for (entity, mut task) in &mut tasks {
        if let Some(result) = check_ready(&mut task.0) {
            commands.queue(move |world: &mut World| {
                if let Ok(mut entity_world) = world.get_entity_mut(entity) {
                    entity_world.remove::<ComputingAltitude>();
                    entity_world.insert(AltitudeArtifact {
                        altitude_map: result.altitude_map,
                        biome_map: result.biome_map,
                    });
                }
            });
        }
    }
}

fn queue_base_terrain_tasks(
    mut commands: Commands,
    target_chunks: Query<(Entity, &TerrainChunk, &TerrainColumnRef), With<WaitForBaseTerrain>>,
    computing: Query<(), With<ComputingBaseTerrain>>,
    world_gen_config: Res<WorldGenConfig>,
    column_artifacts: Query<&AltitudeArtifact>,
    render_distance_params: Res<RenderDistanceParams>,
) {
    let thread_pool = AsyncComputeTaskPool::get();
    let config = world_gen_config.biome_registry.clone();

    let available = MAX_BASE_TERRAIN_TASKS_IN_FLIGHT.saturating_sub(computing.iter().count());
    for (entity, terrain_chunk, column_ref) in target_chunks.iter()
        .k_smallest_by_key(available, |(_, chunk, _)| {
            (chunk.position - render_distance_params.player_chunk).length_squared()
        })
    {
        let chunk_pos = terrain_chunk.position;
        
        if !is_within_active_chunk_range(chunk_pos, &render_distance_params) {
            continue;
        }

        if let Ok(artifact) = column_artifacts.get(column_ref.0) {
            let altitude_map = artifact.altitude_map.clone();
            let biome_map = artifact.biome_map.clone();
            let config = config.clone();

            let task = thread_pool.spawn(async move {
                let chunk_data = generation::generate_base_terrain(chunk_pos, &altitude_map, &biome_map, &config);

                BaseTerrainTaskResult { chunk_data }
            });
            commands.queue(move |world: &mut World| {
                if let Ok(mut entity_world) = world.get_entity_mut(entity) {
                    entity_world.remove::<WaitForBaseTerrain>();
                    entity_world.insert(ComputingBaseTerrain(task));
                }
            });
        }
    }
}

fn handle_base_terrain_tasks(
    mut commands: Commands,
    mut tasks: Query<(Entity, &mut ComputingBaseTerrain)>,
    mut revisions: Query<&mut ChunkContentRevision>,
    mut chunk_map: ResMut<ChunkMap>,
) {
    for (entity, mut task) in &mut tasks {
        if let Some(result) = check_ready(&mut task.0) {
            chunk_map.insert(result.chunk_data);
            if let Ok(mut revision) = revisions.get_mut(entity) {
                revision.advance();
            }
            commands.queue(move |world: &mut World| {
                if let Ok(mut entity_world) = world.get_entity_mut(entity) {
                    entity_world.remove::<ComputingBaseTerrain>();
                    entity_world.insert((ChunkBaseTerrainReady, WaitForNeighbors));
                }
            });
        }
    }
}

fn queue_feature_tasks(
    mut commands: Commands,
    target_chunks: Query<(Entity, &TerrainChunk, &TerrainColumnRef), With<WaitForNeighbors>>,
    computing: Query<(), With<ComputingFeatures>>,
    column_artifacts: Query<&AltitudeArtifact>,
    world_gen_config: Res<WorldGenConfig>,
    render_distance_params: Res<RenderDistanceParams>,
    chunk_entities: Res<ChunkEntities>,
    base_terrain_ready: Query<(), With<ChunkBaseTerrainReady>>,
) {
    let thread_pool = AsyncComputeTaskPool::get();
    let config = world_gen_config.biome_registry.clone();
    let seed = world_gen_config.seed;

    let available = MAX_FEATURE_TASKS_IN_FLIGHT.saturating_sub(computing.iter().count());
    for (entity, terrain_chunk, column_ref) in target_chunks.iter()
        .k_smallest_by_key(available, |(_, chunk, _)| {
            (chunk.position - render_distance_params.player_chunk).length_squared()
        })
    {
        let chunk_pos = terrain_chunk.position;
        
        if !is_within_active_chunk_range(chunk_pos, &render_distance_params) {
            continue;
        }

        // Check neighbors (3x3 area in XZ plane)
        let mut all_neighbors_ready = true;
        for dx in -1..=1 {
            for dz in -1..=1 {
                if dx == 0 && dz == 0 { continue; }
                let neighbor_pos = chunk_pos + IVec3::new(dx, 0, dz);
                let neighbor_ready = chunk_entities.entities.get(&neighbor_pos)
                    .is_some_and(|entity| base_terrain_ready.get(*entity).is_ok());
                if !neighbor_ready {
                    all_neighbors_ready = false;
                    break;
                }
            }
        }

        if all_neighbors_ready {
            let Ok(artifact) = column_artifacts.get(column_ref.0) else {
                continue;
            };

            let config = config.clone();
            let altitude_map = artifact.altitude_map.clone();
            let biome_map = artifact.biome_map.clone();

            let task = thread_pool.spawn(async move {
                let changes = generation::generate_features(chunk_pos, seed, &altitude_map, &biome_map, &config);

                FeaturesTaskResult { changes }
            });
            
            commands.queue(move |world: &mut World| {
                if let Ok(mut entity_world) = world.get_entity_mut(entity) {
                    entity_world.remove::<WaitForNeighbors>();
                    entity_world.insert(ComputingFeatures(task));
                }
            });
        }
    }
}

fn handle_feature_tasks(
    mut commands: Commands,
    mut tasks: Query<(Entity, &mut ComputingFeatures)>,
    chunk_entities: Res<ChunkEntities>,
    mut chunk_map: ResMut<ChunkMap>,
    mut revisions: Query<&mut ChunkContentRevision>,
) {
    for (entity, mut task) in &mut tasks {
        if let Some(result) = check_ready(&mut task.0) {
            apply_voxel_changes(
                &mut commands,
                &mut chunk_map,
                &chunk_entities,
                &mut revisions,
                result.changes,
                crate::voxel_world::storage::VoxelWritePolicy::ReplaceSoft,
            );

            commands.queue(move |world: &mut World| {
                if let Ok(mut entity_world) = world.get_entity_mut(entity) {
                    entity_world.remove::<ComputingFeatures>();
                    entity_world.insert(ChunkGenerationComplete);
                }
            });
        }
    }
}

fn emit_chunk_generated_events(
    chunks: Query<&TerrainChunk, Added<ChunkGenerationComplete>>,
    mut event_writer: MessageWriter<ChunkGeneratedEvent>,
) {
    for chunk in &chunks {
        event_writer.write(ChunkGeneratedEvent(chunk.position));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunks_in_the_same_xz_share_one_terrain_column() {
        let mut app = App::new();
        app.insert_resource(TerrainColumns::default())
            .add_systems(Update, ensure_terrain_columns);

        let lower = app.world_mut().spawn((
            TerrainChunk { position: IVec3::new(3, 0, -2) },
            WaitForTerrainGeneration,
        )).id();
        let upper = app.world_mut().spawn((
            TerrainChunk { position: IVec3::new(3, 4, -2) },
            WaitForTerrainGeneration,
        )).id();

        app.update();

        let lower_column = app.world().get::<TerrainColumnRef>(lower).unwrap().0;
        let upper_column = app.world().get::<TerrainColumnRef>(upper).unwrap().0;
        assert_eq!(lower_column, upper_column);
        assert_eq!(app.world().resource::<TerrainColumns>().entities.len(), 1);
        assert!(app.world().get::<WaitForAltitude>(lower_column).is_some());
    }
}
