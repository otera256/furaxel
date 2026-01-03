use bevy::prelude::*;
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::render::render_resource::AsBindGroup;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct WaterExtension {
    #[uniform(100)]
    pub deep_color: LinearRgba,
    #[uniform(100)]
    pub shallow_color: LinearRgba,
    #[uniform(100)]
    pub depth_scale: f32,
    // パディング
    #[uniform(100)]
    pub _padding: Vec3,
}

impl Default for WaterExtension {
    fn default() -> Self {
        Self {
            deep_color: LinearRgba::from(Color::srgba(0.0, 0.05, 0.5, 0.99)),
            shallow_color: LinearRgba::from(Color::srgba(0.05, 0.1, 0.4, 0.0)),
            depth_scale: 0.5,
            _padding: Vec3::ZERO,
        }
    }
}

impl MaterialExtension for WaterExtension {
    fn fragment_shader() -> bevy::shader::ShaderRef {
        "shaders/water_material.wgsl".into()
    }

    fn prepass_fragment_shader() -> bevy::shader::ShaderRef {
        bevy::shader::ShaderRef::Default
    }
}

pub type WaterMaterial = ExtendedMaterial<StandardMaterial, WaterExtension>;