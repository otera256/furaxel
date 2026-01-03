use bevy::prelude::*;
use block_mesh::{MergeVoxel, Voxel as MergableVoxel, VoxelVisibility};

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
                #[allow(dead_code)]
                pub const $name: Self = Self { id: $id };
            )*

            #[allow(dead_code)]
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
    // System
    EMPTY = 0 => {
        visibility: VoxelVisibility::Empty,
        material: VoxelMaterial::None
    },
    DEBUG = 1 => {
        visibility: VoxelVisibility::Opaque,
        material: VoxelMaterial::Uniform(MaterialDef::texture("textures/default.png"))
    },

    // Terrain (Ground)
    DIRT = 2 => {
        visibility: VoxelVisibility::Opaque,
        material: VoxelMaterial::Uniform(MaterialDef::texture("textures/dirt.png").with_reflectance(0.05))
    },
    COARSE_DIRT = 3 => {
        visibility: VoxelVisibility::Opaque,
        material: VoxelMaterial::Uniform(MaterialDef::color(Color::srgb(0.4, 0.3, 0.2)).with_roughness(0.9))
    },
    GRASS = 4 => {
        visibility: VoxelVisibility::Opaque,
        material: VoxelMaterial::Column {
            top: MaterialDef::texture("textures/grass_top.png").with_reflectance(0.2),
            side: MaterialDef::texture("textures/grass_side.png"),
            bottom: MaterialDef::texture("textures/dirt.png").with_reflectance(0.05)
        }
    },
    STONE = 5 => {
        visibility: VoxelVisibility::Opaque,
        material: VoxelMaterial::Uniform(MaterialDef::texture("textures/stone.png"))
    },
    COBBLESTONE = 6 => {
        visibility: VoxelVisibility::Opaque,
        material: VoxelMaterial::Uniform(MaterialDef::color(Color::srgb(0.5, 0.5, 0.5)).with_roughness(0.8))
    },
    GRAVEL = 7 => {
        visibility: VoxelVisibility::Opaque,
        material: VoxelMaterial::Uniform(MaterialDef::color(Color::srgb(0.4, 0.4, 0.4)))
    },
    SAND = 8 => {
        visibility: VoxelVisibility::Opaque,
        material: VoxelMaterial::Uniform(MaterialDef::texture("textures/sand.png"))
    },
    RED_SAND = 9 => {
        visibility: VoxelVisibility::Opaque,
        material: VoxelMaterial::Uniform(MaterialDef::color(Color::srgb(0.8, 0.4, 0.1)))
    },
    MUD = 10 => {
        visibility: VoxelVisibility::Opaque,
        material: VoxelMaterial::Uniform(MaterialDef::color(Color::srgb(0.3, 0.2, 0.1)).with_roughness(0.8))
    },
    CLAY = 11 => {
        visibility: VoxelVisibility::Opaque,
        material: VoxelMaterial::Uniform(MaterialDef::color(Color::srgb(0.7, 0.4, 0.3)).with_roughness(0.8))
    },
    SNOW = 12 => {
        visibility: VoxelVisibility::Opaque,
        material: VoxelMaterial::Uniform(MaterialDef::color(Color::srgb(0.95, 0.95, 1.0)).with_roughness(0.5))
    },
    ICE = 13 => {
        visibility: VoxelVisibility::Translucent,
        material: VoxelMaterial::Uniform(MaterialDef::color(Color::srgba(0.6, 0.8, 1.0, 0.8)).with_roughness(0.1).with_alpha_mode(AlphaMode::Blend))
    },
    PACKED_ICE = 14 => {
        visibility: VoxelVisibility::Opaque,
        material: VoxelMaterial::Uniform(MaterialDef::color(Color::srgb(0.7, 0.8, 1.0)).with_roughness(0.2))
    },

    // Liquid
    WATER = 15 => {
        visibility: VoxelVisibility::Translucent,
        material: VoxelMaterial::Water(MaterialDef::color(Color::srgba(0.0, 0.3, 0.8, 0.5)).with_roughness(0.1).with_alpha_mode(AlphaMode::Blend))
    },

    // Trees
    OAK_LOG = 16 => {
        visibility: VoxelVisibility::Opaque,
        material: VoxelMaterial::Uniform(MaterialDef::color(Color::srgb(0.4, 0.25, 0.1)).with_roughness(0.8))
    },
    OAK_LEAVES = 17 => {
        visibility: VoxelVisibility::Translucent,
        material: VoxelMaterial::Uniform(MaterialDef::color(Color::srgba(0.1, 0.6, 0.1, 0.8)).with_roughness(0.8).with_alpha_mode(AlphaMode::Blend))
    },
    PINE_LOG = 18 => {
        visibility: VoxelVisibility::Opaque,
        material: VoxelMaterial::Uniform(MaterialDef::color(Color::srgb(0.3, 0.2, 0.1)).with_roughness(0.8))
    },
    PINE_LEAVES = 19 => {
        visibility: VoxelVisibility::Translucent,
        material: VoxelMaterial::Uniform(MaterialDef::color(Color::srgba(0.05, 0.3, 0.05, 0.8)).with_roughness(0.8).with_alpha_mode(AlphaMode::Blend))
    },
    BIRCH_LOG = 20 => {
        visibility: VoxelVisibility::Opaque,
        material: VoxelMaterial::Uniform(MaterialDef::color(Color::srgb(0.9, 0.9, 0.8)).with_roughness(0.8))
    },
    BIRCH_LEAVES = 21 => {
        visibility: VoxelVisibility::Translucent,
        material: VoxelMaterial::Uniform(MaterialDef::color(Color::srgba(0.4, 0.8, 0.4, 0.8)).with_roughness(0.8).with_alpha_mode(AlphaMode::Blend))
    },
    ACACIA_LOG = 22 => {
        visibility: VoxelVisibility::Opaque,
        material: VoxelMaterial::Uniform(MaterialDef::color(Color::srgb(0.45, 0.4, 0.35)).with_roughness(0.8))
    },
    ACACIA_LEAVES = 23 => {
        visibility: VoxelVisibility::Translucent,
        material: VoxelMaterial::Uniform(MaterialDef::color(Color::srgba(0.3, 0.5, 0.1, 0.8)).with_roughness(0.8).with_alpha_mode(AlphaMode::Blend))
    },
    JUNGLE_LOG = 24 => {
        visibility: VoxelVisibility::Opaque,
        material: VoxelMaterial::Uniform(MaterialDef::color(Color::srgb(0.35, 0.2, 0.05)).with_roughness(0.8))
    },
    JUNGLE_LEAVES = 25 => {
        visibility: VoxelVisibility::Translucent,
        material: VoxelMaterial::Uniform(MaterialDef::color(Color::srgba(0.1, 0.7, 0.1, 0.8)).with_roughness(0.8).with_alpha_mode(AlphaMode::Blend))
    },
    CHERRY_LOG = 26 => {
        visibility: VoxelVisibility::Opaque,
        material: VoxelMaterial::Uniform(MaterialDef::color(Color::srgb(0.4, 0.1, 0.1)).with_roughness(0.8))
    },
    CHERRY_LEAVES = 27 => {
        visibility: VoxelVisibility::Translucent,
        material: VoxelMaterial::Uniform(MaterialDef::color(Color::srgba(0.9, 0.4, 0.6, 0.8)).with_roughness(0.8).with_alpha_mode(AlphaMode::Blend))
    },

    // Plants
    CACTUS = 28 => {
        visibility: VoxelVisibility::Opaque,
        material: VoxelMaterial::Uniform(MaterialDef::color(Color::srgb(0.1, 0.5, 0.1)).with_roughness(0.8))
    },
    BAMBOO = 29 => {
        visibility: VoxelVisibility::Opaque,
        material: VoxelMaterial::Uniform(MaterialDef::color(Color::srgb(0.3, 0.7, 0.2)).with_roughness(0.5))
    },
    FLOWER_RED = 30 => {
        visibility: VoxelVisibility::Empty,
        material: VoxelMaterial::Cross(MaterialDef::texture("textures/flower_red.png"))
    },
    FLOWER_YELLOW = 31 => {
        visibility: VoxelVisibility::Empty,
        material: VoxelMaterial::Cross(MaterialDef::texture("textures/flower_yellow.png"))
    },
    TALL_GRASS = 32 => {
        visibility: VoxelVisibility::Empty,
        material: VoxelMaterial::Cross(MaterialDef::color(Color::srgba(0.2, 0.6, 0.2, 0.0)))
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