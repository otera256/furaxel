pub mod biomes;
pub mod storage;
mod feature;
pub mod generation;

use bevy::{
    ecs::world::CommandQueue, prelude::*, tasks::{AsyncComputeTaskPool, Task, futures::check_ready}
};
use itertools::Itertools;
use std::sync::Arc;
use crate::voxel_world::{
    chunk_map::ChunkMap, chunking::{RenderDistanceParams, TerrainChunk}, terrain_generation::storage::TerrainGenerationStorage
};
use self::biomes::BiomeRegistry;

pub struct TerrainGenerationPlugin;

#[derive(Event, Message, Debug, Clone, Copy)]
pub struct ChunkGeneratedEvent(pub IVec3);

impl Plugin for TerrainGenerationPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(TerrainGenerationStorage::default())
            .add_systems(Startup, setup_terrain_generation)
            .add_systems(Update, (
                queue_altitude_tasks,
                handle_altitude_tasks,
                queue_base_terrain_tasks,
                handle_base_terrain_tasks,
                queue_feature_tasks,
                handle_feature_tasks,
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

#[derive(Component, Debug)]
struct ComputingAltitude(Task<CommandQueue>);

#[derive(Component)]
struct WaitForBaseTerrain;

#[derive(Component, Debug)]
struct ComputingBaseTerrain(Task<CommandQueue>);

#[derive(Component, Debug)]
struct WaitForNeighbors;

#[derive(Component, Debug)]
struct ComputingFeatures(Task<CommandQueue>);

const MAX_COMPUTE_TERRAIN_TASKS_PER_FRAME: usize = 10;

fn queue_altitude_tasks(
    mut commands: Commands,
    render_distance_params: Res<RenderDistanceParams>,
    target_chunks: Query<(Entity, &TerrainChunk), With<WaitForTerrainGeneration>>,
    world_gen_config: Res<WorldGenConfig>,
    storage: Res<TerrainGenerationStorage>,
) {
    let thread_pool = AsyncComputeTaskPool::get();
    let config = world_gen_config.biome_registry.clone();
    let seed = world_gen_config.seed;
    let player_chunk = render_distance_params.player_chunk;
    let h = render_distance_params.horizontal;
    let v = render_distance_params.vertical;

    for (entity, terrain_chunk) in target_chunks
        .iter()
        .k_smallest_by_key(MAX_COMPUTE_TERRAIN_TASKS_PER_FRAME, |(_, chunk)| {
            (chunk.position - render_distance_params.player_chunk).xz().length_squared()
        })
    {
        let chunk_pos = terrain_chunk.position;
        
        // Check if chunk is too far
        let offset = chunk_pos - player_chunk;
        if offset.y.abs() > v + 2 || offset.x * offset.x + offset.z * offset.z > (h + 2) * (h + 2) {
            continue;
        }

        let chunk_xz = chunk_pos.xz();

        if storage.altitude_maps.contains_key(&chunk_xz) {
            // Already computed, skip to base terrain
            commands.entity(entity)
                .remove::<WaitForTerrainGeneration>()
                .insert(WaitForBaseTerrain);
        } else {
            // Need to compute altitude map
            let config = config.clone();
            let task = thread_pool.spawn(async move {
                let mut command_queue = CommandQueue::default();

                let (altitude_map, biome_map) = generation::generate_altitude_map(seed, chunk_xz, &config);

                let altitude_arc: Arc<[i32]> = altitude_map.into();
                let biome_arc: Arc<[u8]> = biome_map.into();
                
                command_queue.push(move |world: &mut World| {
                    let mut storage = world.resource_mut::<TerrainGenerationStorage>();
                    storage.altitude_maps.insert(chunk_xz, altitude_arc);
                    storage.biome_maps.insert(chunk_xz, biome_arc);
                });
                command_queue
            });
            commands.entity(entity)
                .remove::<WaitForTerrainGeneration>()
                .insert(ComputingAltitude(task));
        }
    }
}

fn handle_altitude_tasks(
    mut commands: Commands,
    mut tasks: Query<(Entity, &mut ComputingAltitude)>,
) {
    for (entity, mut task) in &mut tasks {
        if let Some(mut command_queue) = check_ready(&mut task.0) {
            commands.append(&mut command_queue);
            commands.entity(entity)
                .remove::<ComputingAltitude>()
                .insert(WaitForBaseTerrain);
        }
    }
}

fn queue_base_terrain_tasks(
    mut commands: Commands,
    target_chunks: Query<(Entity, &TerrainChunk), With<WaitForBaseTerrain>>,
    world_gen_config: Res<WorldGenConfig>,
    storage: Res<TerrainGenerationStorage>,
    render_distance_params: Res<RenderDistanceParams>,
) {
    let thread_pool = AsyncComputeTaskPool::get();
    let config = world_gen_config.biome_registry.clone();
    let player_chunk = render_distance_params.player_chunk;
    let h = render_distance_params.horizontal;
    let v = render_distance_params.vertical;

    for (entity, terrain_chunk) in target_chunks.iter() {
        let chunk_pos = terrain_chunk.position;
        
        // Check if chunk is too far (same logic as update_chunk_entities removal)
        let offset = chunk_pos - player_chunk;
        if offset.y.abs() > v + 2 || offset.x * offset.x + offset.z * offset.z > (h + 2) * (h + 2) {
            continue;
        }

        let chunk_xz = chunk_pos.xz();

        if let Some(altitude_map) = storage.altitude_maps.get(&chunk_xz) {
            let altitude_map = altitude_map.clone();
            let biome_map = storage.biome_maps.get(&chunk_xz).unwrap().clone();
            let config = config.clone();

            let task = thread_pool.spawn(async move {
                let mut command_queue = CommandQueue::default();
                
                let chunk_data = generation::generate_base_terrain(chunk_pos, &altitude_map, &biome_map, &config);

                command_queue.push(move |world: &mut World| {
                    world.resource_mut::<ChunkMap>().insert(chunk_data);
                    world.resource_mut::<TerrainGenerationStorage>().base_terrain_generated.insert(chunk_pos);
                });
                command_queue
            });
            commands.entity(entity)
                .remove::<WaitForBaseTerrain>()
                .insert(ComputingBaseTerrain(task));
        }
    }
}

fn handle_base_terrain_tasks(
    mut commands: Commands,
    mut tasks: Query<(Entity, &mut ComputingBaseTerrain)>,
) {
    for (entity, mut task) in &mut tasks {
        if let Some(mut command_queue) = check_ready(&mut task.0) {
            commands.append(&mut command_queue);
            commands.entity(entity)
                .remove::<ComputingBaseTerrain>()
                .insert(WaitForNeighbors);
        }
    }
}

fn queue_feature_tasks(
    mut commands: Commands,
    target_chunks: Query<(Entity, &TerrainChunk), With<WaitForNeighbors>>,
    storage: Res<TerrainGenerationStorage>,
    world_gen_config: Res<WorldGenConfig>,
    render_distance_params: Res<RenderDistanceParams>,
) {
    let thread_pool = AsyncComputeTaskPool::get();
    let config = world_gen_config.biome_registry.clone();
    let seed = world_gen_config.seed;
    let player_chunk = render_distance_params.player_chunk;
    let h = render_distance_params.horizontal;
    let v = render_distance_params.vertical;

    for (entity, terrain_chunk) in target_chunks.iter() {
        let chunk_pos = terrain_chunk.position;
        
        // Check if chunk is too far
        let offset = chunk_pos - player_chunk;
        if offset.y.abs() > v + 2 || offset.x * offset.x + offset.z * offset.z > (h + 2) * (h + 2) {
            continue;
        }

        let chunk_xz = chunk_pos.xz();
        
        // Check neighbors (3x3 area in XZ plane)
        let mut all_neighbors_ready = true;
        for dx in -1..=1 {
            for dz in -1..=1 {
                if dx == 0 && dz == 0 { continue; }
                let neighbor_pos = chunk_pos + IVec3::new(dx, 0, dz);
                if !storage.base_terrain_generated.contains(&neighbor_pos) {
                    all_neighbors_ready = false;
                    break;
                }
            }
        }

        if all_neighbors_ready {
            let config = config.clone();
            let altitude_map = storage.altitude_maps.get(&chunk_xz).unwrap().clone();
            let biome_map = storage.biome_maps.get(&chunk_xz).unwrap().clone();

            let task = thread_pool.spawn(async move {
                let mut command_queue = CommandQueue::default();
                
                let changes = generation::generate_features(chunk_pos, seed, &altitude_map, &biome_map, &config);

                command_queue.push(move |world: &mut World| {
                    world.resource_mut::<ChunkMap>().set_bulk(changes);
                });
                command_queue
            });
            
            commands.entity(entity)
                .remove::<WaitForNeighbors>()
                .insert(ComputingFeatures(task));
        }
    }
}

fn handle_feature_tasks(
    mut commands: Commands,
    mut tasks: Query<(Entity, &mut ComputingFeatures, &TerrainChunk)>,
    mut event_writer: MessageWriter<ChunkGeneratedEvent>,
    mut storage: ResMut<TerrainGenerationStorage>,
) {
    for (entity, mut task, terrain_chunk) in &mut tasks {
        if let Some(mut command_queue) = check_ready(&mut task.0) {
            commands.append(&mut command_queue);
            commands.entity(entity)
                .remove::<ComputingFeatures>();
            
            event_writer.write(ChunkGeneratedEvent(terrain_chunk.position));
            storage.fully_generated.insert(terrain_chunk.position);
        }
    }
}
