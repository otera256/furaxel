mod voxel_world;

use bevy::prelude::*;
use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};
use bevy_flycam::prelude::*;
use itertools::iproduct;

use crate::voxel_world::{VoxelWorldPlugin, chunk_map::ChunkMap, meshing::MaterialRepository, player::Player, terrain_chunk::{TERRAIN_CHUNK_LENGTH, TerrainChunkData}, voxel::Voxel};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            EguiPlugin::default(),
            WorldInspectorPlugin::new(),
            NoCameraPlayerPlugin,
            VoxelWorldPlugin
        ))
        .add_systems(PostStartup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut chunk_map: ResMut<ChunkMap>,
    material_repo: Res<MaterialRepository>,
) {
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
        Transform::from_translation(Vec3::ZERO)
            .looking_at(Vec3::new(-0.15, -0.05, 0.25), Vec3::Y),
    ));

    // テスト用に複数のチャンクにまたがる地形を生成
    let dirt_voxel = Voxel::new(2);
    let grass_voxel = Voxel::new(3);
    let chunk_range = -4..4;
    for (x, y, z) in iproduct!(chunk_range.clone(), chunk_range.clone(), chunk_range.clone()) {
        let chunk_pos = IVec3::new(x, y, z);
        let terrain_chunk = TerrainChunkData::new_from_fn(
            chunk_pos,
            |world_pos: IVec3| {
                if world_pos.y < 0 {
                    dirt_voxel
                } else if world_pos.y == 0 {
                    grass_voxel
                } else {
                    Voxel::EMPTY
                }
            },
        );
        chunk_map.insert(terrain_chunk);
    }
    // チャンクメッシュを生成してスポーン
    for (x, y, z) in iproduct!(chunk_range.clone(), chunk_range.clone(), chunk_range.clone()) {
        let chunk_pos = IVec3::new(x, y, z);
        if let Some(padded_voxels) = chunk_map.get_padded_chunk_vec(&chunk_pos) {
            for (handle, mesh) in material_repo.create_mesh(padded_voxels) {
                commands.spawn((
                    Mesh3d(meshes.add(mesh)),
                    MeshMaterial3d(handle),
                    Transform::from_translation(chunk_pos.as_vec3() * TERRAIN_CHUNK_LENGTH),
                ));
            }
        }
    }
}