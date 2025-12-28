pub mod voxel;
pub mod meshing;
pub mod chunk;
pub mod terrain_chunk;
pub mod chunk_map;
pub mod chunking;
pub mod player;

use bevy::prelude::*;
use chunking::*;
use chunk_map::*;
use player::*;
pub struct VoxelWorldPlugin;

impl Plugin for VoxelWorldPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(RenderDistanceParams::default())
            .insert_resource(ChunkEntities::default())
            .insert_resource(ChunkMap::default())
            .add_systems(PreUpdate, update_player_chunk)
            .add_systems(Update, test_player_chunk_update.run_if(resource_changed::<RenderDistanceParams>))
            ;
    }
}

fn test_player_chunk_update(
    render_distance_params: Res<RenderDistanceParams>,
) {
    info!("Player chunk position: {:?}", render_distance_params.player_chunk);
}