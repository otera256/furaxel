use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use bevy::tasks::futures::check_ready;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use crate::voxel_world::{
    chunk_map::ChunkMap,
    chunking::TerrainChunk,
};
use super::material::{MaterialRepository, VoxelMaterialHandle};

// メッシュが作成中または既に作成されたチャンクに付与されるコンポーネント
#[derive(Component)]
pub struct MeshQueued;

#[derive(Component)]
pub struct NeedMeshUpdate;

#[derive(Component)]
pub struct NeedImmediateMeshUpdate;

#[derive(Component)]
pub struct TerrainMesh;

#[derive(Component)]
pub struct ComputingMesh(Task<Vec<(VoxelMaterialHandle, Mesh)>>);

pub fn queue_mesh_tasks(
    mut commands: Commands,
    chunk_map: Res<ChunkMap>,
    material_repo: Res<MaterialRepository>,
    chunks: Query<(Entity, &TerrainChunk), (With<NeedMeshUpdate>, Without<ComputingMesh>, Without<NeedImmediateMeshUpdate>)>,
) {
    let thread_pool = AsyncComputeTaskPool::get();

    for (entity, chunk) in chunks.iter() {
        if let Some(padded_chunk) = chunk_map.get_padded_chunk_vec(&chunk.position) {
            let material_repo = material_repo.clone();
            let task = thread_pool.spawn(async move {
                material_repo.create_mesh(padded_chunk)
            });
            commands.entity(entity)
                .remove::<NeedMeshUpdate>()
                .insert(ComputingMesh(task))
                .insert(MeshQueued);
        }
    }
}

pub fn handle_mesh_tasks(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut tasks: Query<(Entity, &mut ComputingMesh, Option<&Children>)>,
) {
    for (entity, mut task, children) in &mut tasks {
        if let Some(generated_meshes) = check_ready(&mut task.0) {
            commands.entity(entity).remove::<ComputingMesh>();
            
            // Despawn old meshes
            if let Some(children) = children {
                for child in children.iter() {
                    commands.entity(child).despawn();
                }
            }
            
            // Spawn new meshes
            commands.entity(entity).with_children(|parent| {
                for (material, mesh) in generated_meshes {
                    match material {
                        VoxelMaterialHandle::Standard(handle) => {
                            parent.spawn((
                                Mesh3d(meshes.add(mesh)),
                                MeshMaterial3d(handle),
                                TerrainMesh,
                            ));
                        },
                        VoxelMaterialHandle::Water(handle) => {
                            parent.spawn((
                                Mesh3d(meshes.add(mesh)),
                                MeshMaterial3d(handle),
                                TerrainMesh,
                                NotShadowCaster
                            ));
                        },
                    }
                }
            });
        }
    }
}

pub fn immediate_mesh_update(
    mut commands: Commands,
    chunk_map: Res<ChunkMap>,
    material_repo: Res<MaterialRepository>,
    mut meshes: ResMut<Assets<Mesh>>,
    chunks: Query<(Entity, &TerrainChunk, Option<&Children>), With<NeedImmediateMeshUpdate>>,
) {
    for (entity, chunk, children) in chunks.iter() {
        if let Some(padded_chunk) = chunk_map.get_padded_chunk_vec(&chunk.position) {
            let generated_meshes = material_repo.create_mesh(padded_chunk);
            
            // Despawn old meshes
            if let Some(children) = children {
                for child in children.iter() {
                    commands.entity(child).despawn();
                }
            }

            commands.entity(entity).with_children(|parent| {
                for (material, mesh) in generated_meshes {
                    match material {
                        VoxelMaterialHandle::Standard(handle) => {
                            parent.spawn((
                                Mesh3d(meshes.add(mesh)),
                                MeshMaterial3d(handle),
                                TerrainMesh,
                            ));
                        },
                        VoxelMaterialHandle::Water(handle) => {
                            parent.spawn((
                                Mesh3d(meshes.add(mesh)),
                                MeshMaterial3d(handle),
                                TerrainMesh,
                                NotShadowCaster
                            ));
                        },
                    }
                }
            });
        }
        commands.entity(entity)
            .remove::<NeedImmediateMeshUpdate>()
            .remove::<NeedMeshUpdate>() // Also remove NeedMeshUpdate if present
            .insert(MeshQueued);
    }
}