use bevy::{asset::RenderAssetUsages, ecs::relationship::RelatedSpawnerCommands, image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor}, light::NotShadowCaster, mesh::{Indices, PrimitiveTopology}, platform::collections::HashMap, prelude::*};
use block_mesh::{Axis, GreedyQuadsBuffer, MergeVoxel, OrientedBlockFace, RIGHT_HANDED_Y_UP_CONFIG, UnorientedQuad, VoxelVisibility, greedy_quads, ndshape::Shape};
use itertools::Itertools;
use crate::voxel_world::{chunk::Chunk, rendering::water::WaterExtension, voxel::{self, VOXEL_SIZE, VoxelMaterial}};
use super::water::WaterMaterial;

#[derive(Component)]
pub struct TerrainMesh;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VoxelMaterialHandle {
    Standard(Handle<StandardMaterial>),
    Water(Handle<WaterMaterial>),
}

impl VoxelMaterialHandle {
    pub fn spawn(&self, parent: &mut RelatedSpawnerCommands<'_, ChildOf>, mesh: Handle<Mesh>) {
        match self {
            VoxelMaterialHandle::Standard(handle) => {
                parent.spawn((
                    Mesh3d(mesh),
                    MeshMaterial3d(handle.clone()),
                    TerrainMesh,
                ));
            },
            VoxelMaterialHandle::Water(handle) => {
                parent.spawn((
                    Mesh3d(mesh),
                    MeshMaterial3d(handle.clone()),
                    TerrainMesh,
                    NotShadowCaster,
                ));
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoxelMeshKind {
    Cube,
    Cross,
    Water,
}

#[derive(Debug, Resource, Clone)]
pub struct MaterialRepository {
    // if default_material is shown, some error occurred
    pub default_material: Handle<StandardMaterial>,
    pub materials: Vec<[VoxelMaterialHandle; 6]>,
    pub visibilities: Vec<VoxelVisibility>,
    pub voxel_kinds: Vec<VoxelMeshKind>,
}

impl Default for MaterialRepository {
    fn default() -> Self {
        Self {
            default_material: Handle::default(),
            materials: Vec::new(),
            visibilities: Vec::new(),
            voxel_kinds: Vec::new(),
        }
    }
}

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

struct MeshBuilder{
    quads: Vec<(OrientedBlockFace, UnorientedQuad)>,
}

impl MeshBuilder {
    fn new(quads: Vec<(OrientedBlockFace, UnorientedQuad)>) -> Self {
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

impl MaterialRepository {
    pub fn set_material(&mut self, id: u16, handles: [VoxelMaterialHandle; 6], visibility: VoxelVisibility, kind: VoxelMeshKind) {
        let id = id as usize;
        if id >= self.materials.len() {
            self.materials.resize(id + 1, std::array::from_fn(|_| VoxelMaterialHandle::Standard(self.default_material.clone())));
            self.visibilities.resize(id + 1, VoxelVisibility::Empty);
            self.voxel_kinds.resize(id + 1, VoxelMeshKind::Cube);
        }
        self.materials[id] = handles;
        self.visibilities[id] = visibility;
        self.voxel_kinds[id] = kind;
    }

    pub fn get_material_handle(&self, material_index: usize, face_index: usize) -> VoxelMaterialHandle {
        if material_index >= self.materials.len() {
            VoxelMaterialHandle::Standard(self.default_material.clone())
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

    pub fn get_voxel_kind(&self, voxel_id: u16) -> VoxelMeshKind {
        if (voxel_id as usize) < self.voxel_kinds.len() {
            self.voxel_kinds[voxel_id as usize]
        } else {
            VoxelMeshKind::Cube
        }
    }

    pub fn create_mesh<S: Shape<3, Coord = u32>>(&self, chunk: Chunk<S>) -> Vec<(VoxelMaterialHandle, Mesh)> {
        let mut meshing_voxels = Vec::with_capacity(chunk.voxels.len());
        let mut cross_voxels = Vec::new();

        for (i, v) in chunk.voxels.iter().enumerate() {
            let kind = self.get_voxel_kind(v.id);
            match kind {
                VoxelMeshKind::Cross => {
                    meshing_voxels.push(MeshingVoxel {
                        id: v.id,
                        visibility: VoxelVisibility::Empty,
                    });
                    cross_voxels.push((i, v.id));
                },
                _ => {
                    meshing_voxels.push(MeshingVoxel {
                        id: v.id,
                        visibility: self.get_visibility(v.id),
                    });
                }
            }
        }

        let mut meshes = self.generate_greedy_mesh(&chunk, &meshing_voxels);
        meshes.extend(self.generate_cross_mesh(&chunk, &cross_voxels));
        meshes
    }

    fn generate_greedy_mesh<S: Shape<3, Coord = u32>>(&self, chunk: &Chunk<S>, meshing_voxels: &[MeshingVoxel]) -> Vec<(VoxelMaterialHandle, Mesh)> {
        let faces = RIGHT_HANDED_Y_UP_CONFIG.faces;
        let mut buffer = GreedyQuadsBuffer::new(chunk.voxels.len());
        let dims = chunk.shape.as_array();
        let min = [0, 0, 0];
        let max = [dims[0] - 1, dims[1] - 1, dims[2] - 1];

        greedy_quads(
            meshing_voxels,
            &chunk.shape,
            min,
            max,
            &faces,
            &mut buffer,
        );

        buffer.quads.groups.into_iter()
            .zip(faces.into_iter().enumerate())
            .flat_map(|(quads, (face_i, face))| {
                let chunk = &chunk;
                quads.into_iter().map(move |quad| {
                    let local_pos = quad.minimum;
                    let voxel = chunk.get_at(UVec3 { x: local_pos[0], y: local_pos[1], z: local_pos[2] });
                    let handle = self.get_material_handle(voxel.id as usize, face_i);
                    (handle, (face, quad))
                })
            })
            .into_group_map()
            .into_iter()
            .map(|(handle, quads)| (handle, MeshBuilder::new(quads).get_mesh()))
            .collect()
    }

    fn generate_cross_mesh<S: Shape<3, Coord = u32>>(&self, chunk: &Chunk<S>, cross_voxels: &[(usize, u16)]) -> Vec<(VoxelMaterialHandle, Mesh)> {
        let mut cross_groups: HashMap<VoxelMaterialHandle, (Vec<u32>, Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<[f32; 2]>)> = HashMap::new();
        let dims = chunk.shape.as_array();
        let min = [0, 0, 0];
        let max = [dims[0] - 1, dims[1] - 1, dims[2] - 1];

        // Define the two diagonal planes relative to the voxel origin (0,0,0) to (1,1,1)
        let planes = [
            (
                [[0.0, 0.0, 0.0], [1.0, 0.0, 1.0], [1.0, 1.0, 1.0], [0.0, 1.0, 0.0]], // Positions
                [-0.70710678, 0.0, 0.70710678] // Normal
            ),
            (
                [[0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [0.0, 1.0, 1.0]], // Positions
                [0.70710678, 0.0, 0.70710678] // Normal
            )
        ];
        
        let uvs_pattern = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
        let indices_pattern = [0, 1, 2, 0, 2, 3];

        for &(index, voxel_id) in cross_voxels {
            let pos_arr = chunk.shape.delinearize(index as u32);
            if pos_arr[0] <= min[0] || pos_arr[0] >= max[0] ||
               pos_arr[1] <= min[1] || pos_arr[1] >= max[1] ||
               pos_arr[2] <= min[2] || pos_arr[2] >= max[2] {
                continue;
            }
            let pos = UVec3::new(pos_arr[0], pos_arr[1], pos_arr[2]).as_vec3() * VOXEL_SIZE;
            let handle = self.get_material_handle(voxel_id as usize, 0);
            
            let (indices, positions, normals, uvs) = cross_groups.entry(handle).or_insert_with(|| (Vec::new(), Vec::new(), Vec::new(), Vec::new()));
            
            for (plane_positions, normal) in &planes {
                let start_index = positions.len() as u32;
                
                for &p in plane_positions {
                    positions.push([
                        pos.x + p[0] * VOXEL_SIZE,
                        pos.y + p[1] * VOXEL_SIZE,
                        pos.z + p[2] * VOXEL_SIZE
                    ]);
                    normals.push(*normal);
                }
                
                uvs.extend_from_slice(&uvs_pattern);
                
                for &i in &indices_pattern {
                    indices.push(start_index + i);
                }
            }
        }

        cross_groups.into_iter().map(|(handle, (indices, positions, normals, uvs))| {
            let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD)
                .with_inserted_indices(Indices::U32(indices))
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
                .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
                .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
            (handle, mesh)
        }).collect()
    }
}

pub fn material_setup(
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut water_materials: ResMut<Assets<WaterMaterial>>,
    mut material_repo: ResMut<MaterialRepository>,
    asset_server: Res<AssetServer>,
) {
    let loading_settings = |s: &mut ImageLoaderSettings| {
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
        base_color_texture: Some(asset_server.load_with_settings("textures/default.png", loading_settings)),
        ..default()
    });

    material_repo.default_material = default_material.clone();

    let definitions = voxel::get_voxel_definitions();

    for (id, visibility, material_def) in definitions {
        let (handles, kind) = create_voxel_material_handles(
            material_def,
            &mut materials,
            &mut water_materials,
            &asset_server,
            &material_repo.default_material,
        );
        material_repo.set_material(id, handles, visibility, kind);
    }
}

fn create_voxel_material_handles(
    def: VoxelMaterial,
    materials: &mut Assets<StandardMaterial>,
    water_materials: &mut Assets<WaterMaterial>,
    asset_server: &AssetServer,
    default_material: &Handle<StandardMaterial>,
) -> ([VoxelMaterialHandle; 6], VoxelMeshKind) {
    let loading_settings = |s: &mut ImageLoaderSettings| {
        *s = ImageLoaderSettings {
            sampler: ImageSampler::Descriptor(ImageSamplerDescriptor {
                address_mode_u: ImageAddressMode::Repeat,
                address_mode_v: ImageAddressMode::Repeat,
                ..default()
            }),
            ..default()
        }
    };

    match def {
        VoxelMaterial::None => (
            std::array::from_fn(|_| VoxelMaterialHandle::Standard(default_material.clone())),
            VoxelMeshKind::Cube
        ),
        VoxelMaterial::Uniform(def) => {
            let material = create_standard_material(materials, asset_server, def, loading_settings);
            (std::array::from_fn(|_| VoxelMaterialHandle::Standard(material.clone())), VoxelMeshKind::Cube)
        },
        VoxelMaterial::Column { top, side, bottom } => {
            let top_mat = create_standard_material(materials, asset_server, top, loading_settings);
            let side_mat = create_standard_material(materials, asset_server, side, loading_settings);
            let bottom_mat = create_standard_material(materials, asset_server, bottom, loading_settings);
            ([
                VoxelMaterialHandle::Standard(side_mat.clone()),
                VoxelMaterialHandle::Standard(bottom_mat),
                VoxelMaterialHandle::Standard(side_mat.clone()),
                VoxelMaterialHandle::Standard(side_mat.clone()),
                VoxelMaterialHandle::Standard(top_mat),
                VoxelMaterialHandle::Standard(side_mat)
            ], VoxelMeshKind::Cube)
        },
        VoxelMaterial::Cross(def) => {
            let mut def = def;
            def.alpha_mode = AlphaMode::Mask(0.5);
            let texture = def.texture.map(|path| asset_server.load_with_settings(path, loading_settings));
            let handle = materials.add(StandardMaterial {
                base_color: def.base_color,
                base_color_texture: texture,
                perceptual_roughness: def.perceptual_roughness,
                reflectance: def.reflectance,
                alpha_mode: def.alpha_mode,
                cull_mode: None, // Double sided
                double_sided: true,
                ..default()
            });
            (std::array::from_fn(|_| VoxelMaterialHandle::Standard(handle.clone())), VoxelMeshKind::Cross)
        },
        VoxelMaterial::Water(def) => {
            let material = water_materials.add(WaterMaterial {
                base: StandardMaterial { 
                    base_color: Color::linear_rgba(0.0, 0.5, 1.0, 0.2),
                    base_color_texture: def.texture.map(|path| asset_server.load_with_settings(path, loading_settings)),
                    perceptual_roughness: 0.08,
                    metallic: 0.1,
                    reflectance: 1.0,
                    alpha_mode: AlphaMode::Blend,
                    ..default()
                 },
                extension: WaterExtension::default(),
            });
            (std::array::from_fn(|_| VoxelMaterialHandle::Water(material.clone())), VoxelMeshKind::Water)
        },
    }
}

fn create_standard_material(
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
