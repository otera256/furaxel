use block_mesh::{Axis, GreedyQuadsBuffer, OrientedBlockFace, RIGHT_HANDED_Y_UP_CONFIG, UnorientedQuad, greedy_quads, ndshape::{ConstShape, Shape}};
use bevy::{asset::RenderAssetUsages, mesh::{Indices, PrimitiveTopology}, prelude::*};
use itertools::Itertools;

use crate::voxel_world::{chunk::Chunk, terrain_chunk::PaddedTerrainChunkShape, voxel::{VOXEL_SIZE, Voxel}};

#[derive(Debug, Resource)]
pub struct MaterialRepository {
    // if default_material is shown, some error occurred
    pub default_material: Handle<StandardMaterial>,
    pub materials: Vec<[Handle<StandardMaterial>; 6]>,
}

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
                    uvs.extend_from_slice(&face.tex_coords(Axis::X, false, quad));
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
