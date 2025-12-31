pub mod meshing;
pub mod material;

use bevy::prelude::*;

use crate::voxel_world::rendering::{meshing::*, material::*};

pub struct VoxelRenderingPlugin;

impl Plugin for VoxelRenderingPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(MaterialRepository::default())
            .add_systems(Startup, material_setup)
            .add_systems(Update, (
                queue_mesh_tasks,
                handle_mesh_tasks,
                immediate_mesh_update,
            ));
    }
}
