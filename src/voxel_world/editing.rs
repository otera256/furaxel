use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

pub use crate::voxel_world::edit_store::{VoxelEditPersistence, VoxelEditStore};

use crate::voxel_world::{
    core::{ChunkContentRevision, ChunkEntities, ChunkMeshDirty, Voxel},
    edit_store::{load_voxel_edits, save_dirty_voxel_edits},
    player::Player,
    raycast::raycast_chunk_map,
    storage::{ChunkMap, VoxelMutationReport, VoxelWritePolicy},
};

pub struct VoxelEditingPlugin;

impl Plugin for VoxelEditingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VoxelEditStore>()
            .init_resource::<VoxelEditPersistence>()
            .init_resource::<VoxelEditorState>()
            .add_message::<VoxelEditRequest>()
            .add_systems(Startup, load_voxel_edits)
            .add_systems(
                Update,
                (
                    select_editor_voxel,
                    emit_mouse_voxel_edits,
                    apply_requested_voxel_edits,
                    save_dirty_voxel_edits,
                )
                    .chain(),
            );
    }
}

#[derive(Resource, Debug, Clone, Copy)]
pub struct VoxelEditorState {
    pub selected_voxel: Voxel,
    pub reach: f32,
}

impl Default for VoxelEditorState {
    fn default() -> Self {
        Self {
            selected_voxel: Voxel::STONE,
            reach: 8.0,
        }
    }
}

/// Public entry point for gameplay voxel edits.
///
/// Sending a request guarantees that successful changes advance the owning
/// chunk revision and invalidate every loaded mesh that observes the changed
/// voxel.
#[derive(Message, Debug, Clone)]
pub struct VoxelEditRequest {
    pub changes: Vec<(IVec3, Voxel)>,
}

impl VoxelEditRequest {
    #[allow(dead_code)]
    pub fn single(position: IVec3, voxel: Voxel) -> Self {
        Self {
            changes: vec![(position, voxel)],
        }
    }

    #[allow(dead_code)]
    pub fn batch(changes: Vec<(IVec3, Voxel)>) -> Self {
        Self { changes }
    }
}

fn select_editor_voxel(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mut state: ResMut<VoxelEditorState>,
) {
    let Some(keys) = keys else {
        return;
    };
    let selected = if keys.just_pressed(KeyCode::Digit1) {
        Some(Voxel::DIRT)
    } else if keys.just_pressed(KeyCode::Digit2) {
        Some(Voxel::STONE)
    } else if keys.just_pressed(KeyCode::Digit3) {
        Some(Voxel::GRASS)
    } else if keys.just_pressed(KeyCode::Digit4) {
        Some(Voxel::COBBLESTONE)
    } else {
        None
    };
    if let Some(selected) = selected {
        state.selected_voxel = selected;
    }
}

fn emit_mouse_voxel_edits(
    mouse_buttons: Option<Res<ButtonInput<MouseButton>>>,
    chunk_map: Res<ChunkMap>,
    state: Res<VoxelEditorState>,
    cameras: Query<&Transform, (With<Player>, With<Camera3d>)>,
    cursor_options: Query<&CursorOptions, With<PrimaryWindow>>,
    mut requests: MessageWriter<VoxelEditRequest>,
) {
    let Some(mouse_buttons) = mouse_buttons else {
        return;
    };
    let Some(cursor_options) = cursor_options.iter().next() else {
        return;
    };
    if cursor_options.grab_mode == CursorGrabMode::None {
        return;
    }
    let Some(transform) = cameras.iter().next() else {
        return;
    };
    if !mouse_buttons.just_pressed(MouseButton::Left)
        && !mouse_buttons.just_pressed(MouseButton::Right)
    {
        return;
    }

    let forward = transform.forward();
    let direction = Vec3::new(forward.x, forward.y, forward.z);
    let Some(hit) = raycast_chunk_map(&chunk_map, transform.translation, direction, state.reach)
    else {
        return;
    };

    if mouse_buttons.just_pressed(MouseButton::Left) {
        requests.write(VoxelEditRequest::single(hit.position, Voxel::EMPTY));
    } else if let Some(position) = hit.previous_empty_position {
        requests.write(VoxelEditRequest::single(position, state.selected_voxel));
    }
}

pub(crate) fn apply_voxel_changes(
    commands: &mut Commands,
    chunk_map: &mut ChunkMap,
    chunk_entities: &ChunkEntities,
    revisions: &mut Query<&mut ChunkContentRevision>,
    changes: Vec<(IVec3, Voxel)>,
    policy: VoxelWritePolicy,
) -> VoxelMutationReport {
    let report = chunk_map.apply_voxel_changes(changes, policy);

    for chunk_pos in &report.changed_chunks {
        if let Some(entity) = chunk_entities.entities.get(chunk_pos)
            && let Ok(mut revision) = revisions.get_mut(*entity)
        {
            revision.advance();
        }
    }

    for chunk_pos in &report.mesh_dirty_chunks {
        if let Some(entity) = chunk_entities.entities.get(chunk_pos) {
            commands.entity(*entity).insert(ChunkMeshDirty);
        }
    }

    report
}

fn apply_requested_voxel_edits(
    mut commands: Commands,
    mut requests: MessageReader<VoxelEditRequest>,
    mut chunk_map: ResMut<ChunkMap>,
    chunk_entities: Res<ChunkEntities>,
    mut revisions: Query<&mut ChunkContentRevision>,
    mut edit_store: ResMut<VoxelEditStore>,
) {
    for request in requests.read() {
        // Record first: edits aimed at an unloaded chunk must not be lost.
        edit_store.record(&request.changes);
        apply_voxel_changes(
            &mut commands,
            &mut chunk_map,
            &chunk_entities,
            &mut revisions,
            request.changes.clone(),
            VoxelWritePolicy::Always,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voxel_world::core::TerrainChunkData;
    use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

    #[test]
    fn edit_request_advances_revision_and_marks_mesh_dirty() {
        let mut app = App::new();
        app.add_plugins(VoxelEditingPlugin)
            .insert_resource(VoxelEditPersistence { path: None })
            .insert_resource(ChunkMap::default())
            .insert_resource(ChunkEntities::default())
            .insert_resource(ButtonInput::<MouseButton>::default())
            .insert_resource(ButtonInput::<KeyCode>::default());

        let entity = app.world_mut().spawn(ChunkContentRevision::default()).id();
        app.world_mut()
            .resource_mut::<ChunkEntities>()
            .entities
            .insert(IVec3::ZERO, entity);
        app.world_mut()
            .resource_mut::<ChunkMap>()
            .insert(TerrainChunkData::new_empty(IVec3::ZERO));
        app.world_mut().write_message(VoxelEditRequest::single(
            IVec3::new(1, 1, 1),
            Voxel::STONE,
        ));

        app.update();

        assert_eq!(app.world().get::<ChunkContentRevision>(entity).unwrap().0, 1);
        assert!(app.world().get::<ChunkMeshDirty>(entity).is_some());
        assert_eq!(
            app.world().resource::<ChunkMap>().get_at(IVec3::new(1, 1, 1)),
            Some(Voxel::STONE),
        );
        assert_eq!(
            app.world()
                .resource::<VoxelEditStore>()
                .changes_for_chunk(IVec3::ZERO),
            vec![(IVec3::new(1, 1, 1), Voxel::STONE)],
        );
    }

    #[test]
    fn edit_to_an_unloaded_chunk_is_retained() {
        let mut app = App::new();
        app.add_plugins(VoxelEditingPlugin)
            .insert_resource(VoxelEditPersistence { path: None })
            .insert_resource(ChunkMap::default())
            .insert_resource(ChunkEntities::default())
            .insert_resource(ButtonInput::<MouseButton>::default())
            .insert_resource(ButtonInput::<KeyCode>::default());

        let position = IVec3::new(-65, 130, 7);
        app.world_mut()
            .write_message(VoxelEditRequest::single(position, Voxel::STONE));
        app.update();

        assert_eq!(app.world().resource::<ChunkMap>().get_at(position), None);
        assert_eq!(
            app.world()
                .resource::<VoxelEditStore>()
                .changes_for_chunk(IVec3::new(-2, 2, 0)),
            vec![(position, Voxel::STONE)],
        );
    }

    #[test]
    fn left_click_ray_hit_emits_a_remove_request() {
        let mut app = App::new();
        app.add_plugins(VoxelEditingPlugin)
            .insert_resource(VoxelEditPersistence { path: None })
            .insert_resource(ChunkMap::default())
            .insert_resource(ChunkEntities::default())
            .insert_resource(ButtonInput::<MouseButton>::default())
            .insert_resource(ButtonInput::<KeyCode>::default());

        let chunk_entity = app
            .world_mut()
            .spawn((
                ChunkContentRevision::default(),
                Player,
                Camera3d::default(),
                Transform::from_translation(Vec3::new(0.5, 0.5, 0.5))
                    .looking_to(Vec3::X, Vec3::Y),
            ))
            .id();
        app.world_mut()
            .resource_mut::<ChunkEntities>()
            .entities
            .insert(IVec3::ZERO, chunk_entity);
        app.world_mut()
            .resource_mut::<ChunkMap>()
            .insert(TerrainChunkData::new_from_fn_local(IVec3::ZERO, |position| {
                if position == UVec3::new(3, 0, 0) {
                    Voxel::STONE
                } else {
                    Voxel::EMPTY
                }
            }));

        let mut cursor = CursorOptions::default();
        cursor.grab_mode = CursorGrabMode::Confined;
        app.world_mut().spawn((PrimaryWindow, cursor));
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);

        app.update();

        assert_eq!(
            app.world()
                .resource::<ChunkMap>()
                .get_at(IVec3::new(3, 0, 0)),
            Some(Voxel::EMPTY),
        );
        assert_eq!(
            app.world()
                .resource::<VoxelEditStore>()
                .changes_for_chunk(IVec3::ZERO),
            vec![(IVec3::new(3, 0, 0), Voxel::EMPTY)],
        );
    }

    #[test]
    fn right_click_places_selected_voxel_in_previous_empty_cell() {
        let mut app = App::new();
        app.add_plugins(VoxelEditingPlugin)
            .insert_resource(VoxelEditPersistence { path: None })
            .insert_resource(ChunkMap::default())
            .insert_resource(ChunkEntities::default())
            .insert_resource(ButtonInput::<MouseButton>::default())
            .insert_resource(ButtonInput::<KeyCode>::default());

        let chunk_entity = app
            .world_mut()
            .spawn((
                ChunkContentRevision::default(),
                Player,
                Camera3d::default(),
                Transform::from_translation(Vec3::new(0.5, 0.5, 0.5))
                    .looking_to(Vec3::X, Vec3::Y),
            ))
            .id();
        app.world_mut()
            .resource_mut::<ChunkEntities>()
            .entities
            .insert(IVec3::ZERO, chunk_entity);
        app.world_mut()
            .resource_mut::<ChunkMap>()
            .insert(TerrainChunkData::new_from_fn_local(IVec3::ZERO, |position| {
                if position == UVec3::new(3, 0, 0) {
                    Voxel::STONE
                } else {
                    Voxel::EMPTY
                }
            }));

        let mut cursor = CursorOptions::default();
        cursor.grab_mode = CursorGrabMode::Confined;
        app.world_mut().spawn((PrimaryWindow, cursor));
        app.world_mut()
            .resource_mut::<VoxelEditorState>()
            .selected_voxel = Voxel::DIRT;
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Right);

        app.update();

        assert_eq!(
            app.world()
                .resource::<ChunkMap>()
                .get_at(IVec3::new(2, 0, 0)),
            Some(Voxel::DIRT),
        );
        assert_eq!(
            app.world()
                .resource::<VoxelEditStore>()
                .changes_for_chunk(IVec3::ZERO),
            vec![(IVec3::new(2, 0, 0), Voxel::DIRT)],
        );
    }
}
