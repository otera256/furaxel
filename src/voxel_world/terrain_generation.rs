use bevy::{
    ecs::{world::CommandQueue}, prelude::*, tasks::{AsyncComputeTaskPool, Task, futures::check_ready}
};
use block_mesh::ndshape::{AbstractShape, ConstShape2u32};
use itertools::{Itertools, iproduct};
use noise::{HybridMulti, MultiFractal, NoiseFn, Perlin};
use crate::voxel_world::{
    chunk_map::ChunkMap, chunking::{ChunkEntities, RenderDistanceParams, TerrainChunk}, meshing::NeedMeshUpdate, terrain_chunk::{TERRAIN_CHUNK_SIZE, TerrainChunkData}, voxel::{VOXEL_SIZE, Voxel}
};


#[derive(Component, Debug, Clone, Copy)]
pub struct WaitForTerrainGeneration;

#[derive(Component, Debug)]
pub struct ComputeTerrain(Task<CommandQueue>);

const MAX_COMPUTE_TERRAIN_TASKS_PER_FRAME: usize = 10;
type AltitudeMapShape = ConstShape2u32<
    TERRAIN_CHUNK_SIZE,
    TERRAIN_CHUNK_SIZE,
>;

pub fn spawn_terrain_generation_tasks(
    mut commands: Commands,
    render_distance_params: Res<RenderDistanceParams>,
    target_chunks: Query<(Entity, &TerrainChunk), With<WaitForTerrainGeneration>>,
) {
    let thread_pool = AsyncComputeTaskPool::get();
    // プレイヤーに近いチャンクから順に処理するようにソート
    for (entity, terrain_chunk) in target_chunks
        .iter()
        .k_smallest_by_key(MAX_COMPUTE_TERRAIN_TASKS_PER_FRAME, |(_, chunk)| {
            (chunk.position - render_distance_params.player_chunk).length_squared()
        })
    {
        let chunk_pos = terrain_chunk.position;
        let task = thread_pool.spawn(async move {
            let mut command_queue = CommandQueue::default();
            // ここで重い地形生成処理を行う
            let basic_multi = HybridMulti::<Perlin>::default()
                .set_frequency(0.001)
                .set_lacunarity(2.0)
                .set_persistence(0.5)
                .set_octaves(8);

            let altitude_map = iproduct!(0..TERRAIN_CHUNK_SIZE, 0..TERRAIN_CHUNK_SIZE)
                .map(|(z, x)| {
                    let world_xz = (IVec2::new(x as i32, z as i32)
                        + chunk_pos.xz() * IVec2::splat(TERRAIN_CHUNK_SIZE as i32))
                        .as_vec2() * VOXEL_SIZE;
                    basic_multi.get([world_xz.x as f64, world_xz.y as f64])
                })
                .map(|h| (h * 100.0) as i32 - chunk_pos.y * TERRAIN_CHUNK_SIZE as i32)
                .collect_vec();

            let chunk_data = TerrainChunkData::new_from_fn_local(chunk_pos, |local_pos| {
                let altitude = altitude_map[AltitudeMapShape{}.linearize([local_pos.x, local_pos.z]) as usize];
                let y = local_pos.y as i32;
                if y < altitude {
                    // Dirt
                    Voxel::new(2)
                } else if y == altitude {
                    // Grass
                    Voxel::new(3)
                } else {
                    Voxel::EMPTY
                }
            });
            
            command_queue.push(move |world: &mut World| {
                world.resource_mut::<ChunkMap>().insert(chunk_data);
            });
            command_queue
        });
        commands.entity(entity).insert(ComputeTerrain(task)).remove::<WaitForTerrainGeneration>();
    }
}

pub fn handle_terrain_generation_tasks(
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