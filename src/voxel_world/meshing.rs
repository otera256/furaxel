use block_mesh::{Axis, GreedyQuadsBuffer, MergeVoxel, OrientedBlockFace, RIGHT_HANDED_Y_UP_CONFIG, UnorientedQuad, VoxelVisibility, greedy_quads, ndshape::Shape};
use bevy::{asset::RenderAssetUsages, image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor}, mesh::{Indices, PrimitiveTopology}, prelude::*};
use itertools::Itertools;

use crate::voxel_world::{chunk::Chunk, chunk_map::ChunkMap, chunking::TerrainChunk, voxel::VOXEL_SIZE};

#[derive(Debug, Resource)]
pub struct MaterialRepository {
    // if default_material is shown, some error occurred
    pub default_material: Handle<StandardMaterial>,
    pub materials: Vec<[Handle<StandardMaterial>; 6]>,
    pub visibilities: Vec<VoxelVisibility>,
}

#[allow(dead_code)]
pub enum MaterialType {
    None,
    Uniform{
        material: Handle<StandardMaterial>,
        visibility: VoxelVisibility,
    },
    Column{
        top: Handle<StandardMaterial>,
        side: Handle<StandardMaterial>,
        bottom: Handle<StandardMaterial>,
        visibility: VoxelVisibility,
    },
    PerFace{
        west: Handle<StandardMaterial>,
        bottom: Handle<StandardMaterial>,
        north: Handle<StandardMaterial>,
        east: Handle<StandardMaterial>,
        top: Handle<StandardMaterial>,
        south: Handle<StandardMaterial>,
        visibility: VoxelVisibility,
    }
}

impl Default for MaterialRepository {
    fn default() -> Self {
        Self {
            default_material: Handle::default(),
            materials: Vec::new(),
            visibilities: Vec::new(),
        }
    }
}

impl MaterialRepository {
    pub fn register_material(&mut self, material: MaterialType) -> usize {
        let (handles, visibility) = match material {
            MaterialType::None => {
                (std::array::from_fn(|_| self.default_material.clone()), VoxelVisibility::Empty)
            },
            MaterialType::Uniform { material, visibility } => {
                (std::array::from_fn(|_| material.clone()), visibility)
            },
            MaterialType::Column { top, side, bottom, visibility } => {
                ([side.clone(), bottom.clone(), side.clone(), side.clone(), top.clone(), side.clone()], visibility)
            },
            MaterialType::PerFace { west, bottom, north, east, top, south, visibility } => {
                ([west.clone(), bottom.clone(), north.clone(), east.clone(), top.clone(), south.clone()], visibility)
            },
        };
        self.materials.push(handles);
        self.visibilities.push(visibility);
        self.materials.len() - 1
    }
    pub fn get_material_handle(&self, material_index: usize, face_index: usize) -> Handle<StandardMaterial> {
        if material_index >= self.materials.len() {
            self.default_material.clone()
        } else {
            self.materials[material_index][face_index].clone()
        }
    }

    fn get_visibility(&self, voxel_id: u16) -> VoxelVisibility {
        if (voxel_id as usize) < self.visibilities.len() {
            self.visibilities[voxel_id as usize]
        } else {
            VoxelVisibility::Opaque
        }
    }

    pub fn create_mesh<S: Shape<3, Coord = u32>>(&self, chunk: Chunk<S>) -> Vec<(Handle<StandardMaterial>, Mesh)> {
        let faces = RIGHT_HANDED_Y_UP_CONFIG.faces;

        let mut buffer = GreedyQuadsBuffer::new(chunk.voxels.len());
        
        #[derive(Clone, Copy, PartialEq, Eq)]
        struct MeshingVoxel {
            id: u16,
            visibility: VoxelVisibility,
        }

        impl MergeVoxel for MeshingVoxel {
            type MergeValue = u16;
            fn merge_value(&self) -> Self::MergeValue {
                self.id
            }
        }

        impl block_mesh::Voxel for MeshingVoxel {
            fn get_visibility(&self) -> VoxelVisibility {
                self.visibility
            }
        }

        let meshing_voxels: Vec<MeshingVoxel> = chunk.voxels.iter().map(|v| MeshingVoxel {
            id: v.id,
            visibility: self.get_visibility(v.id),
        }).collect();

        greedy_quads(
            &meshing_voxels,
            &chunk.shape,
            [0; 3],
            chunk.shape.as_array().map(|d| d - 1),
            &faces,
            &mut buffer,
        );

        struct MeshBuilder{
            quads: Vec<(OrientedBlockFace, UnorientedQuad)>,
        }
        impl MeshBuilder {
            fn mew(quads: Vec<(OrientedBlockFace, UnorientedQuad)>) -> Self {
                Self { quads }
            }
            fn get_mesh(&self) -> Mesh {
                let num_indices = self.quads.len() * 6;
                let num_vertices = self.quads.len() * 4;
                let mut indices = Vec::with_capacity(num_indices);
                let mut positions = Vec::with_capacity(num_vertices);
                let mut normals = Vec::with_capacity(num_vertices);
                let mut uvs = Vec::with_capacity(num_vertices);

                for (face, quad) in self.quads.iter() {
                    indices.extend_from_slice(&face.quad_mesh_indices(positions.len() as u32));
                    positions.extend_from_slice(&face.quad_mesh_positions(quad, VOXEL_SIZE));
                    normals.extend_from_slice(&face.quad_mesh_normals());
                    uvs.extend_from_slice(&face.tex_coords(Axis::X, true, quad));
                }

                Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD)
                    .with_inserted_indices(Indices::U32(indices))
                    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
                    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
                    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
            }
        }

        buffer
            .quads
            .groups
            .into_iter()
            .zip(faces.into_iter().enumerate())
            .flat_map(|(quads, (face_i, face))| {
                let chunk = &chunk;
                quads
                    .into_iter()
                    .map(move |quad| {
                        let local_pos = quad.minimum;
                        let voxel = chunk.get_at(UVec3 { x: local_pos[0], y: local_pos[1], z: local_pos[2] });
                        let handle = self.get_material_handle(voxel.id as usize, face_i);
                        (handle, (face, quad))
                })})
            .into_group_map()
            .into_iter()
            .map(|(handle, quads)| (handle, MeshBuilder::mew(quads).get_mesh()))
            .collect()
    }
}

pub fn material_setup(
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
    material_repo.register_material(MaterialType::Uniform { material: default_material, visibility: VoxelVisibility::Opaque });
    // Id 2: Dirt voxel (uniform material)
    let dirt_material = materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load_with_settings("textures/dirt.png", loading_seettings)),
        perceptual_roughness: 0.9,
        reflectance: 0.05,
        ..default()
    });
    material_repo.register_material(MaterialType::Uniform { material: dirt_material.clone(), visibility: VoxelVisibility::Opaque });
    // Id 3: Grass voxel (column material)
    let grass_material_top = materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load_with_settings("textures/grass_top.png", loading_seettings)),
        perceptual_roughness: 0.9,
        reflectance: 0.2,
        ..default()
    });
    let grass_material_side = materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load_with_settings("textures/grass_side.png", loading_seettings)),
        perceptual_roughness: 0.9,
        reflectance: 0.1,
        ..default()
    });
    material_repo.register_material(MaterialType::Column {
        top: grass_material_top,
        side: grass_material_side,
        bottom: dirt_material,
        visibility: VoxelVisibility::Opaque,
    });

    // Id 4: Stone
    let stone_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.5, 0.5),
        perceptual_roughness: 0.8,
        ..default()
    });
    material_repo.register_material(MaterialType::Uniform { material: stone_material, visibility: VoxelVisibility::Opaque });

    // Id 5: Water
    let water_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.0, 0.3, 0.8, 0.5),
        perceptual_roughness: 0.1,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    material_repo.register_material(MaterialType::Uniform { material: water_material, visibility: VoxelVisibility::Translucent });

    // Id 6: Sand
    let sand_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.85, 0.6),
        perceptual_roughness: 0.9,
        ..default()
    });
    material_repo.register_material(MaterialType::Uniform { material: sand_material, visibility: VoxelVisibility::Opaque });

    // Id 7: Wood
    let wood_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.4, 0.25, 0.1),
        perceptual_roughness: 0.8,
        ..default()
    });
    material_repo.register_material(MaterialType::Uniform { material: wood_material, visibility: VoxelVisibility::Opaque });

    // Id 8: Leaves
    let leaves_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.1, 0.6, 0.1, 0.8),
        perceptual_roughness: 0.8,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    material_repo.register_material(MaterialType::Uniform { material: leaves_material, visibility: VoxelVisibility::Translucent });

    // Id 9: Snow
    let snow_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.95, 1.0),
        perceptual_roughness: 0.5,
        ..default()
    });
    material_repo.register_material(MaterialType::Uniform { material: snow_material, visibility: VoxelVisibility::Opaque });

    // Id 10: Gravel
    let gravel_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.4, 0.4, 0.4),
        perceptual_roughness: 0.9,
        ..default()
    });
    material_repo.register_material(MaterialType::Uniform { material: gravel_material, visibility: VoxelVisibility::Opaque });

    // Id 11: Mud
    let mud_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.3, 0.2, 0.1),
        perceptual_roughness: 0.8,
        ..default()
    });
    material_repo.register_material(MaterialType::Uniform { material: mud_material, visibility: VoxelVisibility::Opaque });

    // Id 12: Clay
    let clay_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.7, 0.4, 0.3),
        perceptual_roughness: 0.8,
        ..default()
    });
    material_repo.register_material(MaterialType::Uniform { material: clay_material, visibility: VoxelVisibility::Opaque });

    // Id 13: Cactus
    let cactus_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.1, 0.5, 0.1),
        perceptual_roughness: 0.8,
        ..default()
    });
    material_repo.register_material(MaterialType::Uniform { material: cactus_material, visibility: VoxelVisibility::Opaque });

    // Id 14: Flower Red
    let flower_red_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.1, 0.1),
        perceptual_roughness: 0.8,
        ..default()
    });
    material_repo.register_material(MaterialType::Uniform { material: flower_red_material, visibility: VoxelVisibility::Opaque });

    // Id 15: Flower Yellow
    let flower_yellow_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.9, 0.1),
        perceptual_roughness: 0.8,
        ..default()
    });
    material_repo.register_material(MaterialType::Uniform { material: flower_yellow_material, visibility: VoxelVisibility::Opaque });

    // Id 16: Pine Log
    let pine_log_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.3, 0.2, 0.1),
        perceptual_roughness: 0.8,
        ..default()
    });
    material_repo.register_material(MaterialType::Uniform { material: pine_log_material, visibility: VoxelVisibility::Opaque });

    // Id 17: Pine Leaves
    let pine_leaves_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.05, 0.3, 0.05, 0.8),
        perceptual_roughness: 0.8,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    material_repo.register_material(MaterialType::Uniform { material: pine_leaves_material, visibility: VoxelVisibility::Translucent });

    // Id 18: Birch Log
    let birch_log_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.9, 0.8),
        perceptual_roughness: 0.8,
        ..default()
    });
    material_repo.register_material(MaterialType::Uniform { material: birch_log_material, visibility: VoxelVisibility::Opaque });

    // Id 19: Birch Leaves
    let birch_leaves_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.4, 0.8, 0.4, 0.8),
        perceptual_roughness: 0.8,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    material_repo.register_material(MaterialType::Uniform { material: birch_leaves_material, visibility: VoxelVisibility::Translucent });
}

#[derive(Component)]
pub struct NeedMeshUpdate;

#[derive(Component)]
pub struct TerrainMesh;

pub fn terrain_mesh_update(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    material_repo: Res<MaterialRepository>,
    chunk_map: Res<ChunkMap>,
    query: Query<(Entity, &TerrainChunk, Option<&Children>), With<NeedMeshUpdate>>,
    children_query: Query<Entity, With<TerrainMesh>>,
) {
    for (entity, terrain_chunk, children) in query.iter() {
        // 古いメッシュを削除
        if let Some(children) = children {
            for child in children.iter() {
                if children_query.get(child).is_ok() {
                    commands.entity(child).despawn();
                }
            }
        }
        // 新しいメッシュを生成してスポーン
        if let Some(padded_voxels) = chunk_map.get_padded_chunk_vec(&terrain_chunk.position) {
            for (handle, mesh) in material_repo.create_mesh(padded_voxels) {
                commands.spawn((
                    Mesh3d(meshes.add(mesh)),
                    MeshMaterial3d(handle),
                    TerrainMesh,
                    ChildOf(entity),
                ));
            }
            commands.entity(entity)
                .remove::<NeedMeshUpdate>();
        }
    }
}