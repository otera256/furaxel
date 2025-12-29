use block_mesh::{MergeVoxel, Voxel as MergableVoxel};

pub const VOXEL_SIZE: f32 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Voxel{
    pub id: u16,
}

#[allow(dead_code)]
impl Voxel {
    pub const EMPTY: Self = Self { id: 0 };
    pub const DEBUG: Self = Self { id: 1 };

    pub fn new(id: u16) -> Self {
        Self { id }
    }
}

impl MergableVoxel for Voxel {
    fn get_visibility(&self) -> block_mesh::VoxelVisibility {
        if *self == Self::EMPTY {
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