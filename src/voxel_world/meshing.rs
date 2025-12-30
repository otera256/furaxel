use block_mesh::{Axis, GreedyQuadsBuffer, MergeVoxel, OrientedBlockFace, RIGHT_HANDED_Y_UP_CONFIG, UnorientedQuad, VoxelVisibility, greedy_quads, ndshape::Shape};
use bevy::{asset::RenderAssetUsages, image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor}, mesh::{Indices, PrimitiveTopology}, prelude::*};
use itertools::Itertools;

use crate::voxel_world::{chunk::Chunk, chunk_map::ChunkMap, chunking::TerrainChunk, voxel::{self, VoxelMaterial, VOXEL_SIZE}};

#[derive(Debug, Resource)]
pub struct MaterialRepository {
    // if default_material is shown, some error occurred
    pub default_material: Handle<StandardMaterial>,
    pub materials: Vec<[Handle<StandardMaterial>; 6]>,
    pub visibilities: Vec<VoxelVisibility>,
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
    pub fn set_material(&mut self, id: u16, handles: [Handle<StandardMaterial>; 6], visibility: VoxelVisibility) {
        let id = id as usize;
        if id >= self.materials.len() {
            self.materials.resize(id + 1, std::array::from_fn(|_| self.default_material.clone()));
            self.visibilities.resize(id + 1, VoxelVisibility::Empty);
        }
        self.materials[id] = handles;
        self.visibilities[id] = visibility;
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

    let definitions = voxel::get_voxel_definitions();

    for (id, visibility, material_def) in definitions {
        let handles = match material_def {
            VoxelMaterial::None => {
                std::array::from_fn(|_| material_repo.default_material.clone())
            },
            VoxelMaterial::Uniform(def) => {
                let material = create_material(&mut materials, &asset_server, def, loading_seettings);
                std::array::from_fn(|_| material.clone())
            },
            VoxelMaterial::Column { top, side, bottom } => {
                let top_mat = create_material(&mut materials, &asset_server, top, loading_seettings);
                let side_mat = create_material(&mut materials, &asset_server, side, loading_seettings);
                let bottom_mat = create_material(&mut materials, &asset_server, bottom, loading_seettings);
                [side_mat.clone(), bottom_mat, side_mat.clone(), side_mat.clone(), top_mat, side_mat]
            },
        };
        material_repo.set_material(id, handles, visibility);
    }
}

fn create_material(
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
    def: voxel::MaterialDef,
    loading_settings: impl Fn(&mut ImageLoaderSettings) + Copy + Send + Sync + 'static,
) -> Handle<StandardMaterial> {
    let texture = def.texture.map(|path| asset_server.load_with_settings(path, loading_settings));
    materials.add(StandardMaterial {
        base_color: def.base_color,
        base_color_texture: texture,
        perceptual_roughness: def.perceptual_roughness,
        reflectance: def.reflectance,
        alpha_mode: def.alpha_mode,
        ..default()
    })
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