use std::f32::consts::FRAC_PI_2;

use bevy::{core_pipeline::prepass::DepthPrepass, input::mouse::AccumulatedMouseMotion, pbr::Atmosphere, prelude::*, window::{CursorGrabMode, CursorOptions, PrimaryWindow}};

use crate::voxel_world::core::{RenderDistanceParams, coordinates::TERRAIN_CHUNK_LENGTH};

pub struct VoxelPlayerPlugin;

impl Plugin for VoxelPlayerPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(PlayerSettings::default())
            .add_systems(Startup, setup_player)
            .add_systems(PreUpdate, update_player_chunk)
            .add_systems(Update, (
                player_look,
                player_move,
                toggle_grab_cursor,
            ));
    }
}

fn setup_player(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Player,
        // 水面のレンダリングなどのためにDepthPrepassを有効化
        DepthPrepass,
        DistanceFog {
            color: Color::srgba(0.35, 0.48, 0.66, 1.0),
            directional_light_color: Color::srgba(1.0, 0.95, 0.85, 0.5),
            directional_light_exponent: 30.0,
            falloff: FogFalloff::from_visibility_colors(
                12000.0, // distance in world units up to which objects retain visibility (>= 5% contrast)
                Color::srgb(0.35, 0.5, 0.66), // atmospheric extinction color (after light is lost due to absorption by atmospheric particles)
                Color::srgb(0.8, 0.844, 1.0), // atmospheric inscattering color (light gained due to scattering from the sun)
            ),
        },
        Atmosphere::default(),
        Transform::from_xyz(0.0, 150.0, 0.0).looking_at(Vec3::new(0.0, 150.0, 10.0), Vec3::Y),
    ));
}

// Playerを識別するためのマーカーコンポーネント
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component)]
pub struct Player;

#[derive(Resource)]
pub struct PlayerSettings {
    pub speed: f32,
    pub run_speed: f32,
    pub sensitivity: f32,
}

impl Default for PlayerSettings {
    fn default() -> Self {
        Self {
            speed: 20.0,
            run_speed: 100.0,
            sensitivity: 0.002,
        }
    }
}


pub fn update_player_chunk(
    player_transform: Single<&Transform, With<Player>>,
    mut render_distance_params: ResMut<RenderDistanceParams>,
) {
    let player_pos = player_transform.translation;
    let player_chunk = (player_pos / TERRAIN_CHUNK_LENGTH).floor().as_ivec3();
    // run_if(rsource_changed::<T>) で真の変更のみを検知するために、
    // 明示的に変更があった場合のみ更新する
    if render_distance_params.player_chunk != player_chunk {
        render_distance_params.player_chunk = player_chunk;
    }
}

pub fn player_look(
    mut transform: Single<&mut Transform, With<Player>>,
    accumulated_mouse_motion: Res<AccumulatedMouseMotion>,
    primary_cursor_options: Single<&CursorOptions, With<PrimaryWindow>>,
    settings: Res<PlayerSettings>,
) {
    // カーソルが解放されている場合は視点移動しない
    if primary_cursor_options.grab_mode == CursorGrabMode::None {
        return;
    }
    let delta = accumulated_mouse_motion.delta;

    if delta == Vec2::ZERO {
        return;
    }

    let delta_yaw = -delta.x * settings.sensitivity;
    let delta_pitch = -delta.y * settings.sensitivity;
    let (yaw, pitch, roll) = transform.rotation.to_euler(EulerRot::YXZ);

    let yaw = yaw + delta_yaw;

    const PITCH_LIMIT: f32 = FRAC_PI_2 - 0.01;
    let pitch = (pitch + delta_pitch).clamp(-PITCH_LIMIT, PITCH_LIMIT);

    transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);
}

pub fn player_move(
    mut transform: Single<&mut Transform, With<Player>>,
    keys: Res<ButtonInput<KeyCode>>,
    primary_cursor_options: Single<&CursorOptions, With<PrimaryWindow>>,
    time: Res<Time>,
    settings: Res<PlayerSettings>,
) {
    // カーソルが解放されている場合は移動しない
    if primary_cursor_options.grab_mode == CursorGrabMode::None {
        return;
    }
    let mut velocity = Vec3::ZERO;
    
    // 水平移動のための前方・右方ベクトル（Y成分を無視）
    let forward = transform.forward();
    let right = transform.right();
    
    let flat_forward = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
    let flat_right = Vec3::new(right.x, 0.0, right.z).normalize_or_zero();
    let up = Vec3::Y;

    if keys.pressed(KeyCode::KeyW) {
        velocity += flat_forward;
    }
    if keys.pressed(KeyCode::KeyS) {
        velocity -= flat_forward;
    }
    if keys.pressed(KeyCode::KeyA) {
        velocity -= flat_right;
    }
    if keys.pressed(KeyCode::KeyD) {
        velocity += flat_right;
    }
    if keys.pressed(KeyCode::Space) {
        velocity += up;
    }
    if keys.pressed(KeyCode::ShiftLeft) {
        velocity -= up;
    }

    velocity = velocity.normalize_or_zero();

    let speed = if keys.pressed(KeyCode::ControlLeft) {
        settings.run_speed
    } else {
        settings.speed
    };

    transform.translation = transform.translation + velocity * speed * time.delta_secs();
}

fn toggle_grab_cursor(
    mut primary_cursor_options: Single<&mut CursorOptions, With<PrimaryWindow>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }
    match primary_cursor_options.grab_mode {
        CursorGrabMode::None => {
            primary_cursor_options.grab_mode = CursorGrabMode::Confined;
            primary_cursor_options.visible = false;
        },
        _ => {
            primary_cursor_options.grab_mode = CursorGrabMode::None;
            primary_cursor_options.visible = true;
        }
    }
}