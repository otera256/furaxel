pub mod voxel;
pub mod meshing;
pub mod chunk;
pub mod terrain_chunk;
pub mod chunk_map;
pub mod chunking;
pub mod player;

use bevy::{image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor}, prelude::*};
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
            .add_systems(Startup, setup)
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

fn setup(
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut material_repo: ResMut<MaterialRepository>,
    asset_server: Res<AssetServer>,
) {
    let loading_seettings = |s: &mut _| {
        *s = ImageLoaderSettings {
            sampler: ImageSampler::Descriptor(ImageSamplerDescriptor {
                address_mode_u: ImageAddressMode::Repeat,
                address_mode_v: ImageAddressMode::Repeat,
                ..default()
            }),
            ..default()
        }
    };
    let default_material = materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load_with_settings("textures/default.png", loading_seettings)),
        ..default()
    });

    material_repo.default_material = default_material.clone();

    // Id 0: Empty voxel
    material_repo.register_material(MaterialType::None);
    // Id 1: Debug voxel (uniform material)
    material_repo.register_material(MaterialType::Uniform { material: default_material });
    // Id 2: Dirt voxel
    let dirt_material = materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load_with_settings("textures/dirt.png", loading_seettings)),
        perceptual_roughness: 0.9,
        reflectance: 0.1,
        ..default()
    });
    material_repo.register_material(MaterialType::Uniform { material: dirt_material });
}