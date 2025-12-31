use bevy::prelude::*;
use block_mesh::{MergeVoxel, Voxel as MergableVoxel, VoxelVisibility};

pub const VOXEL_SIZE: f32 = 1.0;

#[derive(Debug, Clone)]
pub struct MaterialDef {
    pub base_color: Color,
    pub texture: Option<&'static str>,
    pub perceptual_roughness: f32,
    pub reflectance: f32,
    pub alpha_mode: AlphaMode,
}

impl Default for MaterialDef {
    fn default() -> Self {
        Self {
            base_color: Color::WHITE,
            texture: None,
            perceptual_roughness: 0.9,
            reflectance: 0.1,
            alpha_mode: AlphaMode::Opaque,
        }
    }
}

impl MaterialDef {
    pub fn color(color: Color) -> Self {
        Self {
            base_color: color,
            ..default()
        }
    }
    pub fn texture(path: &'static str) -> Self {
        Self {
            texture: Some(path),
            ..default()
        }
    }
    pub fn with_roughness(mut self, roughness: f32) -> Self {
        self.perceptual_roughness = roughness;
        self
    }
    pub fn with_reflectance(mut self, reflectance: f32) -> Self {
        self.reflectance = reflectance;
        self
    }
    pub fn with_alpha_mode(mut self, alpha_mode: AlphaMode) -> Self {
        self.alpha_mode = alpha_mode;
        self
    }
}

#[derive(Debug, Clone)]
pub enum VoxelMaterial {
    None,
    Uniform(MaterialDef),
    Column {
        top: MaterialDef,
        side: MaterialDef,
        bottom: MaterialDef,
    },
    Cross(MaterialDef),
    Water(MaterialDef),
}

macro_rules! define_voxels {
    (
        $(
            $name:ident = $id:expr => {
                visibility: $vis:expr,
                material: $mat:expr
            }
        ),* $(,)?
    ) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct Voxel {
            pub id: u16,
        }

        impl Voxel {
            $(
                pub const $name: Self = Self { id: $id };
            )*

            pub fn new(id: u16) -> Self {
                Self { id }
            }

            pub fn visibility(&self) -> VoxelVisibility {
                match self.id {
                    $(
                        $id => $vis,
                    )*
                    _ => VoxelVisibility::Opaque,
                }
            }
        }

        pub fn get_voxel_definitions() -> Vec<(u16, VoxelVisibility, VoxelMaterial)> {
            vec![
                $(
                    ($id, $vis, $mat),
                )*
            ]
        }
    }
}

define_voxels! {
    EMPTY = 0 => {
        visibility: VoxelVisibility::Empty,
        material: VoxelMaterial::None
    },
    DEBUG = 1 => {
        visibility: VoxelVisibility::Opaque,
        material: VoxelMaterial::Uniform(MaterialDef::texture("textures/default.png"))
    },
    DIRT = 2 => {
        visibility: VoxelVisibility::Opaque,
        material: VoxelMaterial::Uniform(MaterialDef::texture("textures/dirt.png").with_reflectance(0.05))
    },
    GRASS = 3 => {
        visibility: VoxelVisibility::Opaque,
        material: VoxelMaterial::Column {
            top: MaterialDef::texture("textures/grass_top.png").with_reflectance(0.2),
            side: MaterialDef::texture("textures/grass_side.png"),
            bottom: MaterialDef::texture("textures/dirt.png").with_reflectance(0.05)
        }
    },
    STONE = 4 => {
        visibility: VoxelVisibility::Opaque,
        material: VoxelMaterial::Uniform(MaterialDef::texture("textures/stone.png"))
    },
    WATER = 5 => {
        visibility: VoxelVisibility::Translucent,
        material: VoxelMaterial::Water(MaterialDef::color(Color::srgba(0.0, 0.3, 0.8, 0.5)).with_roughness(0.1).with_alpha_mode(AlphaMode::Blend))
    },
    SAND = 6 => {
        visibility: VoxelVisibility::Opaque,
        material: VoxelMaterial::Uniform(MaterialDef::texture("textures/sand.png"))
    },
    OAK_LOG = 7 => {
        visibility: VoxelVisibility::Opaque,
        material: VoxelMaterial::Uniform(MaterialDef::color(Color::srgb(0.4, 0.25, 0.1)).with_roughness(0.8))
    },
    OAK_LEAVES = 8 => {
        visibility: VoxelVisibility::Translucent,
        material: VoxelMaterial::Uniform(MaterialDef::color(Color::srgba(0.1, 0.6, 0.1, 0.8)).with_roughness(0.8).with_alpha_mode(AlphaMode::Blend))
    },
    SNOW = 9 => {
        visibility: VoxelVisibility::Opaque,
        material: VoxelMaterial::Uniform(MaterialDef::color(Color::srgb(0.95, 0.95, 1.0)).with_roughness(0.5))
    },
    GRAVEL = 10 => {
        visibility: VoxelVisibility::Opaque,
        material: VoxelMaterial::Uniform(MaterialDef::color(Color::srgb(0.4, 0.4, 0.4)))
    },
    MUD = 11 => {
        visibility: VoxelVisibility::Opaque,
        material: VoxelMaterial::Uniform(MaterialDef::color(Color::srgb(0.3, 0.2, 0.1)).with_roughness(0.8))
    },
    CLAY = 12 => {
        visibility: VoxelVisibility::Opaque,
        material: VoxelMaterial::Uniform(MaterialDef::color(Color::srgb(0.7, 0.4, 0.3)).with_roughness(0.8))
    },
    CACTUS = 13 => {
        visibility: VoxelVisibility::Opaque,
        material: VoxelMaterial::Uniform(MaterialDef::color(Color::srgb(0.1, 0.5, 0.1)).with_roughness(0.8))
    },
    FLOWER_RED = 14 => {
        visibility: VoxelVisibility::Empty,
        material: VoxelMaterial::Cross(MaterialDef::texture("textures/flower_red.png"))
    },
    FLOWER_YELLOW = 15 => {
        visibility: VoxelVisibility::Empty,
        material: VoxelMaterial::Cross(MaterialDef::texture("textures/flower_yellow.png"))
    },
    PINE_LOG = 16 => {
        visibility: VoxelVisibility::Opaque,
        material: VoxelMaterial::Uniform(MaterialDef::color(Color::srgb(0.3, 0.2, 0.1)).with_roughness(0.8))
    },
    PINE_LEAVES = 17 => {
        visibility: VoxelVisibility::Translucent,
        material: VoxelMaterial::Uniform(MaterialDef::color(Color::srgba(0.05, 0.3, 0.05, 0.8)).with_roughness(0.8).with_alpha_mode(AlphaMode::Blend))
    },
    BIRCH_LOG = 18 => {
        visibility: VoxelVisibility::Opaque,
        material: VoxelMaterial::Uniform(MaterialDef::color(Color::srgb(0.9, 0.9, 0.8)).with_roughness(0.8))
    },
    BIRCH_LEAVES = 19 => {
        visibility: VoxelVisibility::Translucent,
        material: VoxelMaterial::Uniform(MaterialDef::color(Color::srgba(0.4, 0.8, 0.4, 0.8)).with_roughness(0.8).with_alpha_mode(AlphaMode::Blend))
    },
    ICE = 20 => {
        visibility: VoxelVisibility::Translucent,
        material: VoxelMaterial::Uniform(MaterialDef::color(Color::srgba(0.6, 0.8, 1.0, 0.8)).with_roughness(0.1).with_alpha_mode(AlphaMode::Blend))
    },
    RED_SAND = 21 => {
        visibility: VoxelVisibility::Opaque,
        material: VoxelMaterial::Uniform(MaterialDef::color(Color::srgb(0.8, 0.4, 0.1)))
    },
    PACKED_ICE = 22 => {
        visibility: VoxelVisibility::Opaque,
        material: VoxelMaterial::Uniform(MaterialDef::color(Color::srgb(0.7, 0.8, 1.0)).with_roughness(0.2))
    },
    BAMBOO = 23 => {
        visibility: VoxelVisibility::Opaque,
        material: VoxelMaterial::Uniform(MaterialDef::color(Color::srgb(0.3, 0.7, 0.2)).with_roughness(0.5))
    },
    ACACIA_LOG = 24 => {
        visibility: VoxelVisibility::Opaque,
        material: VoxelMaterial::Uniform(MaterialDef::color(Color::srgb(0.45, 0.4, 0.35)).with_roughness(0.8))
    },
    ACACIA_LEAVES = 25 => {
        visibility: VoxelVisibility::Translucent,
        material: VoxelMaterial::Uniform(MaterialDef::color(Color::srgba(0.3, 0.5, 0.1, 0.8)).with_roughness(0.8).with_alpha_mode(AlphaMode::Blend))
    },
    JUNGLE_LOG = 26 => {
        visibility: VoxelVisibility::Opaque,
        material: VoxelMaterial::Uniform(MaterialDef::color(Color::srgb(0.35, 0.2, 0.05)).with_roughness(0.8))
    },
    JUNGLE_LEAVES = 27 => {
        visibility: VoxelVisibility::Translucent,
        material: VoxelMaterial::Uniform(MaterialDef::color(Color::srgba(0.1, 0.7, 0.1, 0.8)).with_roughness(0.8).with_alpha_mode(AlphaMode::Blend))
    }
}

impl MergableVoxel for Voxel {
    fn get_visibility(&self) -> VoxelVisibility {
        self.visibility()
    }
}

impl MergeVoxel for Voxel {
    type MergeValue = u16;

    fn merge_value(&self) -> Self::MergeValue {
        self.id
    }
}