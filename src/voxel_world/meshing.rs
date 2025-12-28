use block_mesh::{GreedyQuadsBuffer, RIGHT_HANDED_Y_UP_CONFIG, greedy_quads, ndshape::ConstShape};
use bevy::{asset::RenderAssetUsages, mesh::{Indices, PrimitiveTopology}, prelude::*};

use crate::voxel_world::{terrain_chunk::PaddedTerrainChunkShape, voxel::{VOXEL_SIZE, Voxel}};

// とりあえずPaddedTerrainChunkShapeの形状のみを想定した実装
pub fn crate_terrain_chunk_mesh(voxels: Vec<Voxel>) -> Mesh {
    // 参考: https://github.com/bonsairobo/block-mesh-rs/blob/main/examples-crate/render/main.rs
    let faces = RIGHT_HANDED_Y_UP_CONFIG.faces;

    let mut buffer = GreedyQuadsBuffer::new(voxels.len());
    greedy_quads(
        &voxels,
        &PaddedTerrainChunkShape {},
        [0; 3],
        [
            PaddedTerrainChunkShape::ARRAY[0] - 1,
            PaddedTerrainChunkShape::ARRAY[1] - 1,
            PaddedTerrainChunkShape::ARRAY[2] - 1
        ],
        &faces,
        &mut buffer,
    );

    let num_indices = buffer.quads.num_quads() * 6;
    let num_vertices = buffer.quads.num_quads() * 4;
    let mut indices = Vec::with_capacity(num_indices);
    let mut positions = Vec::with_capacity(num_vertices);
    let mut normals = Vec::with_capacity(num_vertices);

    for (group, face) in buffer.quads.groups.into_iter().zip(faces.into_iter()) {
        for quad in group.into_iter() {
            indices.extend_from_slice(&face.quad_mesh_indices(positions.len() as u32));
            positions.extend_from_slice(&face.quad_mesh_positions(&quad, VOXEL_SIZE));
            normals.extend_from_slice(&face.quad_mesh_normals());
        }
    }

    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD)
        .with_inserted_indices(Indices::U32(indices))
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)

}