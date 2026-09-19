use bevy::{prelude::*, math::IVec3};
use itertools::iproduct;

use crate::voxel_world::{
    core::{
        chunk_range::{is_within_active_chunk_range, should_unload_chunk},
        coordinates::TERRAIN_CHUNK_LENGTH,
        ChunkContentRevision, ChunkEntities, ChunkGenerationComplete, ChunkMeshDirty,
        RenderDistanceParams, TerrainChunk
    },
    storage::ChunkMap,
    pipelines::cpu_noise::WaitForTerrainGeneration
};

pub fn unload_distant_chunks(
    mut chunk_map: ResMut<ChunkMap>,
    render_distance_params: Res<RenderDistanceParams>,
) {
    let mut to_remove = Vec::new();
    for chunk_pos in chunk_map.positions() {
        if should_unload_chunk(*chunk_pos, &render_distance_params) {
            to_remove.push(*chunk_pos);
        }
    }

    if !to_remove.is_empty() {
        // info!("Unloading {} distant chunks", to_remove.len());
        for chunk_pos in to_remove {
            chunk_map.remove(&chunk_pos);
        }
    }
}

pub fn update_chunk_entities(
    mut commands: Commands,
    mut chunk_entities: ResMut<ChunkEntities>,
    render_distance_params: Res<RenderDistanceParams>,
    mut chunk_map: ResMut<ChunkMap>,
    generation_complete: Query<(), With<ChunkGenerationComplete>>,
) {
    let player_chunk = render_distance_params.player_chunk;
    let h = render_distance_params.horizontal;
    let v = render_distance_params.vertical;
    let mut to_create = Vec::new();
    for (x, y, z) in iproduct!(-h..=h, -v..=v, -h..=h) {
        let chunk_pos = player_chunk + IVec3::new(x, y, z);
        if x * x + z * z > h * h {
            continue;
        }
        if !chunk_entities.entities.contains_key(&chunk_pos) {
            to_create.push(chunk_pos);
        }
    }
    // Entity insertion order leaks into several ECS queries. Keeping it near-first
    // gives newly entered areas a sensible order even before their own queues sort.
    to_create.sort_unstable_by_key(|position| (*position - player_chunk).length_squared());
    let mut to_remove = Vec::new();
    // 1チャンク移動するだけで削除と追加を繰り返すのは非効率なので,
    // 削除対象は余裕をもって判定する
    for chunk_pos in chunk_entities.entities.keys() {
        if !is_within_active_chunk_range(*chunk_pos, &render_distance_params) {
            to_remove.push(*chunk_pos);
        }
    }
    // info!("Creating {} chunks, removing {} chunks", to_create.len(), to_remove.len());
    for chunk_pos in to_create {
        let cached = chunk_map.get(&chunk_pos).is_some();
        let mut entity_commands = commands.spawn((
            TerrainChunk {
                position: chunk_pos,
            },
            Transform::from_translation(chunk_pos.as_vec3() * TERRAIN_CHUNK_LENGTH),
            InheritedVisibility::default(),
        ));
        if cached {
            // Cached voxel data is already fully composed (base, features and
            // edit overlay), so only its render artifact must be rebuilt.
            entity_commands.insert((
                ChunkContentRevision(1),
                ChunkGenerationComplete,
                ChunkMeshDirty,
            ));
        } else {
            entity_commands.insert((
                ChunkContentRevision::default(),
                WaitForTerrainGeneration,
            ));
        }
        let entity = entity_commands.id();
        chunk_entities.entities.insert(chunk_pos, entity);
    }
    for chunk_pos in to_remove {
        if let Some(entity) = chunk_entities.entities.remove(&chunk_pos) {
            // Base terrain enters ChunkMap before features do. Never promote
            // that intermediate value to a completed reload cache entry.
            if generation_complete.get(entity).is_err() {
                chunk_map.remove(&chunk_pos);
            }
            commands.entity(entity).despawn_children();
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voxel_world::core::TerrainChunkData;

    #[test]
    fn cached_chunks_resume_at_mesh_generation() {
        let mut app = App::new();
        let mut chunk_map = ChunkMap::default();
        chunk_map.insert(TerrainChunkData::new_empty(IVec3::ZERO));
        app.insert_resource(chunk_map)
            .insert_resource(ChunkEntities::default())
            .insert_resource(RenderDistanceParams {
                player_chunk: IVec3::ZERO,
                horizontal: 0,
                vertical: 0,
            })
            .add_systems(Update, update_chunk_entities);

        app.update();

        let entity = app.world().resource::<ChunkEntities>().entities[&IVec3::ZERO];
        assert!(app.world().get::<ChunkGenerationComplete>(entity).is_some());
        assert!(app.world().get::<ChunkMeshDirty>(entity).is_some());
        assert!(app.world().get::<WaitForTerrainGeneration>(entity).is_none());
    }

    #[test]
    fn uncached_chunks_enter_the_generation_pipeline() {
        let mut app = App::new();
        app.insert_resource(ChunkMap::default())
            .insert_resource(ChunkEntities::default())
            .insert_resource(RenderDistanceParams {
                player_chunk: IVec3::ZERO,
                horizontal: 0,
                vertical: 0,
            })
            .add_systems(Update, update_chunk_entities);

        app.update();

        let entity = app.world().resource::<ChunkEntities>().entities[&IVec3::ZERO];
        assert!(app.world().get::<WaitForTerrainGeneration>(entity).is_some());
        assert!(app.world().get::<ChunkGenerationComplete>(entity).is_none());
    }

    #[test]
    fn incomplete_chunk_data_is_not_kept_as_a_reload_cache_entry() {
        let incomplete_position = IVec3::new(10, 0, 0);
        let mut app = App::new();
        let incomplete_entity = app.world_mut().spawn(TerrainChunk {
            position: incomplete_position,
        }).id();
        let mut chunk_map = ChunkMap::default();
        chunk_map.insert(TerrainChunkData::new_empty(incomplete_position));
        let mut chunk_entities = ChunkEntities::default();
        chunk_entities
            .entities
            .insert(incomplete_position, incomplete_entity);
        app.insert_resource(chunk_map)
            .insert_resource(chunk_entities)
            .insert_resource(RenderDistanceParams {
                player_chunk: IVec3::ZERO,
                horizontal: 0,
                vertical: 0,
            })
            .add_systems(Update, update_chunk_entities);

        app.update();

        assert!(app
            .world()
            .resource::<ChunkMap>()
            .get(&incomplete_position)
            .is_none());
    }

    #[test]
    fn leaving_and_reentering_reuses_completed_voxel_data() {
        let edited_position = IVec3::new(1, 1, 1);
        let mut cached = TerrainChunkData::new_empty(IVec3::ZERO);
        cached.set_at(edited_position, crate::voxel_world::core::Voxel::STONE);

        let mut app = App::new();
        let mut chunk_map = ChunkMap::default();
        chunk_map.insert(cached);
        app.insert_resource(chunk_map)
            .insert_resource(ChunkEntities::default())
            .insert_resource(RenderDistanceParams {
                player_chunk: IVec3::ZERO,
                horizontal: 0,
                vertical: 0,
            })
            .add_systems(Update, update_chunk_entities);

        app.update();
        let original = app.world().resource::<ChunkEntities>().entities[&IVec3::ZERO];

        app.world_mut()
            .resource_mut::<RenderDistanceParams>()
            .player_chunk = IVec3::new(10, 0, 0);
        app.update();
        assert!(!app
            .world()
            .resource::<ChunkEntities>()
            .entities
            .contains_key(&IVec3::ZERO));

        app.world_mut()
            .resource_mut::<RenderDistanceParams>()
            .player_chunk = IVec3::ZERO;
        app.update();

        let reloaded = app.world().resource::<ChunkEntities>().entities[&IVec3::ZERO];
        assert_ne!(reloaded, original);
        assert!(app.world().get::<ChunkGenerationComplete>(reloaded).is_some());
        assert_eq!(
            app.world().resource::<ChunkMap>().get_at(edited_position),
            Some(crate::voxel_world::core::Voxel::STONE),
        );
    }
}
