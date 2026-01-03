pub mod core;
pub mod storage;
pub mod pipelines;
pub mod chunking;
pub mod player;

use bevy::{light::CascadeShadowConfigBuilder, prelude::*};
use bevy::time::common_conditions::on_timer;
use std::time::Duration;
use core::{RenderDistanceParams, ChunkEntities};
use storage::ChunkMap;
use chunking::*;
use player::*;
use pipelines::{cpu_noise::CpuNoiseTerrainGenerationPlugin, cpu_mesh::CpuMeshRenderingPlugin};
use std::marker::PhantomData;

pub type DefaultVoxelWorldPlugin = VoxelWorldPlugin<CpuNoiseTerrainGenerationPlugin, CpuMeshRenderingPlugin>;

pub struct VoxelWorldPlugin<G = CpuNoiseTerrainGenerationPlugin, R = CpuMeshRenderingPlugin> {
    _marker: PhantomData<(G, R)>,
}

impl<G, R> Default for VoxelWorldPlugin<G, R> {
    fn default() -> Self {
        Self { _marker: PhantomData }
    }
}

impl<G: Plugin + Default, R: Plugin + Default> Plugin for VoxelWorldPlugin<G, R> {
    fn build(&self, app: &mut App) {
        app
            .add_plugins((
                G::default(),
                R::default(),
                VoxelPlayerPlugin,
            ))
            .insert_resource(RenderDistanceParams::default())
            .insert_resource(ChunkEntities::default())
            .insert_resource(ChunkMap::default())
            .add_systems(Startup, (
                setup_world,
            ))
            .add_systems(Update, (
                update_chunk_entities.run_if(resource_changed::<RenderDistanceParams>),
            ))
            .add_systems(PostUpdate, (
                unload_distant_chunks.run_if(on_timer(Duration::from_secs(5))),
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
        maximum_distance: 2000.0,
        num_cascades: 3,
        ..default()
    }.build();

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