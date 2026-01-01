pub mod voxel;
pub mod chunk;
pub mod terrain_chunk;
pub mod chunk_map;
pub mod chunking;
pub mod player;
pub mod terrain_generation;
pub mod rendering;

use bevy::{core_pipeline::{prepass::DepthPrepass}, light::CascadeShadowConfigBuilder, pbr::Atmosphere, prelude::*};
use bevy_flycam::{FlyCam, MovementSettings};
use chunking::*;
use chunk_map::*;
use player::*;
use terrain_generation::*;

use crate::voxel_world::rendering::VoxelRenderingPlugin;
pub struct VoxelWorldPlugin;

impl Plugin for VoxelWorldPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins((
                TerrainGenerationPlugin,
                VoxelRenderingPlugin,
            ))
            .insert_resource(RenderDistanceParams::default())
            .insert_resource(ChunkEntities::default())
            .insert_resource(ChunkMap::default())
            .insert_resource(MovementSettings {
                sensitivity: 0.00015,
                speed: 32.0
            })
            .add_systems(Startup, (
                setup_world,
            ))
            .add_systems(PreUpdate, (
                update_player_chunk,
            ))
            .add_systems(Update, (
                update_chunk_entities.run_if(resource_changed::<RenderDistanceParams>),
            ))
            ;
    }
}

fn setup_world(
    mut commands: Commands,
) {
    let cascade_shadow_config = CascadeShadowConfigBuilder {
        first_cascade_far_bound: 10.0,
        minimum_distance: 0.1,
        maximum_distance: 500.0,
        num_cascades: 3,
        ..default()
    }.build();
    // Player Camera
    commands.spawn((
        Camera3d::default(),
        FlyCam,
        Player,
        // 水面のレンダリングなどのためにDepthPrepassを有効化
        DepthPrepass,
        DistanceFog {
            color: Color::srgba(0.35, 0.48, 0.66, 1.0),
            directional_light_color: Color::srgba(1.0, 0.95, 0.85, 0.5),
            directional_light_exponent: 30.0,
            falloff: FogFalloff::from_visibility_colors(
                12000.0, // distance in world units up to which objects retain visibility (>= 5% contrast)
                Color::srgb(0.35, 0.5, 0.66), // atmospheric extinction color (after light is lost due to absorption by atmospheric particles)
                Color::srgb(0.8, 0.844, 1.0), // atmospheric inscattering color (light gained due to scattering from the sun)
            ),
        },
        Atmosphere::default(),
    ));

    // Sun (Light)
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(0.98, 0.95, 0.82),
            shadows_enabled: true,
            illuminance: 20000.0,
            ..default()
        },
        cascade_shadow_config,
        Transform::from_translation(Vec3::ZERO)
            .looking_at(Vec3::new(-0.15, -0.05, 0.25), Vec3::Y),
    ));
}