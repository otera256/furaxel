use block_mesh::{Axis, GreedyQuadsBuffer, OrientedBlockFace, RIGHT_HANDED_Y_UP_CONFIG, UnorientedQuad, greedy_quads, ndshape::Shape};
use bevy::{asset::RenderAssetUsages, image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor}, mesh::{Indices, PrimitiveTopology}, prelude::*};
use itertools::Itertools;

use crate::voxel_world::{chunk::Chunk, chunk_map::ChunkMap, chunking::TerrainChunk, voxel::VOXEL_SIZE};

#[derive(Debug, Resource)]
pub struct MaterialRepository {
    // if default_material is shown, some error occurred
    pub default_material: Handle<StandardMaterial>,
    pub materials: Vec<[Handle<StandardMaterial>; 6]>,
}

#[allow(dead_code)]
pub enum MaterialType {
    None,
    Uniform{
        material: Handle<StandardMaterial>,
    },
    Column{
        top: Handle<StandardMaterial>,
        side: Handle<StandardMaterial>,
        bottom: Handle<StandardMaterial>,
    },
    PerFace{
        west: Handle<StandardMaterial>,
        bottom: Handle<StandardMaterial>,
        north: Handle<StandardMaterial>,
        east: Handle<StandardMaterial>,
        top: Handle<StandardMaterial>,
        south: Handle<StandardMaterial>,
    }
}

impl Default for MaterialRepository {
    fn default() -> Self {
        Self {
            default_material: Handle::default(),
            materials: Vec::new(),
        }
    }
}

impl MaterialRepository {
    pub fn register_material(&mut self, material: MaterialType) -> usize {
        let handles = match material {
            MaterialType::None => {
                std::array::from_fn(|_| self.default_material.clone())
            },
            MaterialType::Uniform { material } => {
                std::array::from_fn(|_| material.clone())
            },
            MaterialType::Column { top, side, bottom } => {
                [side.clone(), bottom.clone(), side.clone(), side.clone(), top.clone(), side.clone()]
            },
            MaterialType::PerFace { west, bottom, north, east, top, south } => {
                [west.clone(), bottom.clone(), north.clone(), east.clone(), top.clone(), south.clone()]
            },
        };
        self.materials.push(handles);
        self.materials.len() - 1
    }
    pub fn get_material_handle(&self, material_index: usize, face_index: usize) -> Handle<StandardMaterial> {
        if material_index >= self.materials.len() {
            self.default_material.clone()
        } else {
            self.materials[material_index][face_index].clone()
        }
    }
    pub fn create_mesh<S: Shape<3, Coord = u32>>(&self, chunk: Chunk<S>) -> Vec<(Handle<StandardMaterial>, Mesh)> {
        let faces = RIGHT_HANDED_Y_UP_CONFIG.faces;

        let mut buffer = GreedyQuadsBuffer::new(chunk.voxels.len());
        
        greedy_quads(
            chunk.as_slice(),
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
    material_repo.register_material(MaterialType::Uniform { material: default_material });
    // Id 2: Dirt voxel (uniform material)
    let dirt_material = materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load_with_settings("textures/dirt.png", loading_seettings)),
        perceptual_roughness: 0.9,
        reflectance: 0.05,
        ..default()
    });
    material_repo.register_material(MaterialType::Uniform { material: dirt_material.clone() });
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
    });
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