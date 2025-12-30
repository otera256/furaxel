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
    pub const DIRT: Self = Self { id: 2 };
    pub const GRASS: Self = Self { id: 3 };
    pub const STONE: Self = Self { id: 4 };
    pub const WATER: Self = Self { id: 5 };
    pub const SAND: Self = Self { id: 6 };
    pub const WOOD: Self = Self { id: 7 };
    pub const LEAVES: Self = Self { id: 8 };
    pub const SNOW: Self = Self { id: 9 };
    pub const GRAVEL: Self = Self { id: 10 };
    pub const MUD: Self = Self { id: 11 };
    pub const CLAY: Self = Self { id: 12 };
    pub const CACTUS: Self = Self { id: 13 };
    pub const FLOWER_RED: Self = Self { id: 14 };
    pub const FLOWER_YELLOW: Self = Self { id: 15 };
    pub const PINE_LOG: Self = Self { id: 16 };
    pub const PINE_LEAVES: Self = Self { id: 17 };
    pub const BIRCH_LOG: Self = Self { id: 18 };
    pub const BIRCH_LEAVES: Self = Self { id: 19 };

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