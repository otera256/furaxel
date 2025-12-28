use block_mesh::{MergeVoxel, Voxel as MergableVoxel};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Voxel{
    pub id: u16,
}

pub const EMPTY_VOXEL: Voxel = Voxel { id: 0 };
pub const DEBUG_VOXEL: Voxel = Voxel { id: 1 };

impl MergableVoxel for Voxel {
    fn get_visibility(&self) -> block_mesh::VoxelVisibility {
        if *self == EMPTY_VOXEL {
            block_mesh::VoxelVisibility::Empty
        } else {
            block_mesh::VoxelVisibility::Opaque
        }
    }
}

impl MergeVoxel for Voxel {
    type MergeValue = u16;

    fn merge_value(&self) -> Self::MergeValue {
        self.id
    }
}