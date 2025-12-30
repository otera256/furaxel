pub mod voxel;
pub mod meshing;
pub mod chunk;
pub mod terrain_chunk;
pub mod chunk_map;
pub mod chunking;
pub mod player;
pub mod terrain_generation;

use bevy::{light::CascadeShadowConfigBuilder, prelude::*};
use bevy_flycam::FlyCam;
use chunking::*;
use chunk_map::*;
use player::*;
use meshing::*;
use terrain_generation::*;
pub struct VoxelWorldPlugin;

impl Plugin for VoxelWorldPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(TerrainGenerationPlugin)
            .insert_resource(RenderDistanceParams::default())
            .insert_resource(ChunkEntities::default())
            .insert_resource(ChunkMap::default())
            .insert_resource(MaterialRepository::default())
            .add_systems(Startup, (
                material_setup,
                setup_world,
            ))
            .add_systems(PreUpdate, (
                update_player_chunk,
            ))
            .add_systems(Update, (
                update_chunk_entities.run_if(resource_changed::<RenderDistanceParams>),
            ))
            .add_systems(PostUpdate, (
                terrain_mesh_update,
            ))
            ;
    }
}

fn setup_world(
    mut commands: Commands,
) {
    let cascade_shadow_config = CascadeShadowConfigBuilder {
        first_cascade_far_bound: 4.0,
        maximum_distance: 300.0,
        num_cascades: 2,
        ..default()
    }.build();
    // Player Camera
    commands.spawn((
        Camera3d::default(),
        FlyCam,
        Player
    ));

    // Sun (Light)
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(0.98, 0.95, 0.82),
            shadows_enabled: true,
            ..default()
        },
        cascade_shadow_config,
        Transform::from_translation(Vec3::ZERO)
            .looking_at(Vec3::new(-0.15, -0.05, 0.25), Vec3::Y),
    ));
}