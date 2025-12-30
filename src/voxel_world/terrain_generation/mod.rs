mod biomes;
mod storage;
mod feature;

use bevy::{
    ecs::world::{self, CommandQueue}, prelude::*, tasks::{AsyncComputeTaskPool, Task, futures::check_ready}
};
use block_mesh::ndshape::{AbstractShape, ConstShape2u32};
use itertools::Itertools;
use noise::{Fbm, RidgedMulti, Billow, Select, ScaleBias, MultiFractal, NoiseFn, Perlin};
use std::sync::Arc;
use crate::voxel_world::{
    chunk_map::ChunkMap, chunking::{ChunkEntities, RenderDistanceParams, TerrainChunk}, meshing::NeedMeshUpdate, terrain_chunk::{TERRAIN_CHUNK_SIZE, TerrainChunkData}, terrain_generation::storage::TerrainGenerationStorage, voxel::{VOXEL_SIZE, Voxel}
};
use self::biomes::BiomeRegistry;

pub struct TerrainGenerationPlugin;

impl Plugin for TerrainGenerationPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(TerrainGenerationStorage::default())
            .add_systems(Startup, setup_terrain_generation)
            .add_systems(Update, (
                spawn_terrain_generation_tasks,
                handle_terrain_generation_tasks,
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
struct ComputeTerrain(Task<CommandQueue>);

const MAX_COMPUTE_TERRAIN_TASKS_PER_FRAME: usize = 10;
type AltitudeMapShape = ConstShape2u32<
    TERRAIN_CHUNK_SIZE,
    TERRAIN_CHUNK_SIZE,
>;

fn spawn_terrain_generation_tasks(
    mut commands: Commands,
    render_distance_params: Res<RenderDistanceParams>,
    target_chunks: Query<(Entity, &TerrainChunk), With<WaitForTerrainGeneration>>,
    world_gen_config: Res<WorldGenConfig>,
) {
    let thread_pool = AsyncComputeTaskPool::get();
    let config = world_gen_config.biome_registry.clone();
    let seed = world_gen_config.seed;

    // プレイヤーに近いチャンクから順に処理するようにソート
    for (entity, terrain_chunk) in target_chunks
        .iter()
        .k_smallest_by_key(MAX_COMPUTE_TERRAIN_TASKS_PER_FRAME, |(_, chunk)| {
            (chunk.position - render_distance_params.player_chunk).length_squared()
        })
    {
        let chunk_pos = terrain_chunk.position;
        let config = config.clone();
        
        let task = thread_pool.spawn(async move {
            let mut command_queue = CommandQueue::default();

            // --- 1. ベース（大陸の形を決める） ---
            // 周波数を低く(0.0005)して、大きな大陸のうねりを作る
            let base_continent = Fbm::<Perlin>::new(seed)
                .set_frequency(0.0005)
                .set_persistence(0.5)
                .set_lacunarity(2.2)
                .set_octaves(6);

            // --- 2. 山岳バイオーム（険しい地形） ---
            // RidgedMultiは鋭い尾根を作るのに適している
            let mountains = RidgedMulti::<Perlin>::new(seed + 1)
                .set_frequency(0.0015) // 少し細かく
                .set_octaves(8);

            // --- 3. 平原バイオーム（なだらかな地形） ---
            // Billowは丸っこい丘を作る。ScaleBiasで高さを抑える(0.125倍)
            let plains_base = Billow::<Perlin>::new(seed + 2)
                .set_frequency(0.001)
                .set_octaves(6);
            let plains = ScaleBias::new(plains_base)
                .set_scale(0.125); // 平原なので高さを低く潰す

            // --- 4. 合成（セレクター） ---
            // base_continent の値に応じて、mountains と plains をブレンドする
            // 境界線（0.0）付近は smooth_falloff の幅で滑らかに混ぜる
            let terrain_generator = Select::new(plains, mountains, base_continent)
                .set_bounds(0.2, 100.0) // ベース値が 0.2 以上なら山にする
                .set_falloff(0.3);      // 境界をなじませる範囲

            // Stack allocation for altitude map to avoid heap allocation
            let mut altitude_map = [0i32; (TERRAIN_CHUNK_SIZE * TERRAIN_CHUNK_SIZE) as usize];

            for z in 0..TERRAIN_CHUNK_SIZE {
                for x in 0..TERRAIN_CHUNK_SIZE {
                    let world_xz = (IVec2::new(x as i32, z as i32)
                        + chunk_pos.xz() * IVec2::splat(TERRAIN_CHUNK_SIZE as i32))
                        .as_vec2() * VOXEL_SIZE;
                    let noise_val = terrain_generator.get([world_xz.x as f64, world_xz.y as f64]);
                    let altitude = (noise_val * 60.0) as i32 - chunk_pos.y * TERRAIN_CHUNK_SIZE as i32;
                    altitude_map[AltitudeMapShape{}.linearize([x, z]) as usize] = altitude;
                }
            }

            let mut chunk_data = TerrainChunkData::new_empty(chunk_pos);

            // Pass 2: Biomes & Surface
            for z in 0..TERRAIN_CHUNK_SIZE {
                for x in 0..TERRAIN_CHUNK_SIZE {
                    let altitude = altitude_map[AltitudeMapShape{}.linearize([x, z]) as usize];
                    
                    let world_x = x as i32 + chunk_pos.x * TERRAIN_CHUNK_SIZE as i32;
                    let world_z = z as i32 + chunk_pos.z * TERRAIN_CHUNK_SIZE as i32;
                    
                    let biome = config.get_biome(world_x as f64, world_z as f64);

                    for y in 0..TERRAIN_CHUNK_SIZE {
                        let local_y = y as i32;
                        let world_y = local_y + chunk_pos.y * TERRAIN_CHUNK_SIZE as i32;
                        let voxel = if local_y < altitude - 3 {
                            Voxel::STONE
                        } else if local_y < altitude {
                            biome.sub_surface_block
                        } else if local_y == altitude {
                            biome.surface_block
                        } else if world_y < 0 {
                            Voxel::WATER
                        } else {
                            Voxel::EMPTY
                        };
                        
                        if voxel != Voxel::EMPTY {
                            *chunk_data.get_local_at_mut(UVec3::new(x, y, z)) = voxel;
                        }
                    }
                }
            }

            // Pass 3: Features
            // Simple feature placement
            for z in 0..TERRAIN_CHUNK_SIZE {
                for x in 0..TERRAIN_CHUNK_SIZE {
                    let altitude = altitude_map[AltitudeMapShape{}.linearize([x, z]) as usize];
                    // Only place features on top of the surface within this chunk
                    if altitude >= 0 && altitude < TERRAIN_CHUNK_SIZE as i32 {
                        let world_x = x as i32 + chunk_pos.x * TERRAIN_CHUNK_SIZE as i32;
                        let world_z = z as i32 + chunk_pos.z * TERRAIN_CHUNK_SIZE as i32;
                        let biome = config.get_biome(world_x as f64, world_z as f64);
                        
                        if !biome.features.is_empty() {
                            // Simple hash for probability
                            let prob_hash = feature::hash(world_x, world_z, seed);
                            let prob = (prob_hash % 10000) as f32 / 10000.0;
                            
                            if prob < biome.feature_probability {
                                // Pick a feature
                                let feature_idx = (prob_hash as usize) % biome.features.len();
                                let feature = &biome.features[feature_idx];
                                feature.place(&mut chunk_data, UVec3::new(x, altitude as u32 + 1, z), seed);
                            }
                        }
                    }
                }
            }
            
            command_queue.push(move |world: &mut World| {
                world.resource_mut::<ChunkMap>().insert(chunk_data);
            });
            command_queue
        });
        commands.entity(entity).insert(ComputeTerrain(task)).remove::<WaitForTerrainGeneration>();
    }
}

fn handle_terrain_generation_tasks(
    mut commands: Commands,
    mut terrain_generation_tasks: Query<(Entity, &mut ComputeTerrain, &TerrainChunk)>,
    chunk_entities: Res<ChunkEntities>
) {
    for (entity, mut compute_terrain, terrain_chunk) in &mut terrain_generation_tasks {
        if let Some(mut command_queue) = check_ready(&mut compute_terrain.0) {
            commands.append(&mut command_queue);
            commands.entity(entity)
                .insert(NeedMeshUpdate)
                .remove::<ComputeTerrain>();
            // 隣接するチャンクもメッシュ更新が必要になる可能性があるのでフラグを立てる
            for offset in [
                    IVec3::new(-1, 0, 0), IVec3::new(1, 0, 0),
                    IVec3::new(0, -1, 0), IVec3::new(0, 1, 0),
                    IVec3::new(0, 0, -1), IVec3::new(0, 0, 1)
                ].into_iter() {
                let neighbor_pos = terrain_chunk.position + offset;
                if let Some(neighbor_entity) = chunk_entities.entities.get(&neighbor_pos) {
                    // 存在するチャンクのみフラグを立てる
                    commands.entity(*neighbor_entity).insert(NeedMeshUpdate);
                }
            }
        }
    }
}