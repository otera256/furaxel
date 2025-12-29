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
use meshing::*;
pub struct VoxelWorldPlugin;

impl Plugin for VoxelWorldPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(RenderDistanceParams::default())
            .insert_resource(ChunkEntities::default())
            .insert_resource(ChunkMap::default())
            .insert_resource(MaterialRepository::default())
            .add_systems(Startup, material_setup)
            .add_systems(PreUpdate, (
                update_player_chunk,
            ))
            .add_systems(Update, update_chunk_entities.run_if(resource_changed::<RenderDistanceParams>))
            ;
    }
}
