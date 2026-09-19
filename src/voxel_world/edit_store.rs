use std::{
    ffi::OsString,
    fs::{self, File},
    io::{self, Cursor, Read, Write},
    path::{Path, PathBuf},
};

use bevy::{platform::collections::HashMap, prelude::*};

use crate::voxel_world::core::{coordinates::TERRAIN_CHUNK_SIZE, Voxel};

const EDIT_FILE_MAGIC: &[u8; 4] = b"FXED";
const EDIT_FILE_VERSION: u32 = 1;
const MAX_SAVED_CHUNKS: u32 = 100_000;
const MAX_EDITS_PER_CHUNK: u32 =
    TERRAIN_CHUNK_SIZE * TERRAIN_CHUNK_SIZE * TERRAIN_CHUNK_SIZE;

#[derive(Debug, Default)]
struct ChunkEditOverlay {
    revision: u64,
    changes: HashMap<UVec3, Voxel>,
}

/// Player-authored voxel state, kept separately from reproducible world generation.
///
/// Entries use chunk-local coordinates so the overlay can be persisted even while
/// its target chunk is not loaded.
#[derive(Resource, Debug, Default)]
pub struct VoxelEditStore {
    chunks: HashMap<IVec3, ChunkEditOverlay>,
    dirty: bool,
}

impl VoxelEditStore {
    pub fn record(&mut self, changes: &[(IVec3, Voxel)]) {
        let chunk_size = IVec3::splat(TERRAIN_CHUNK_SIZE as i32);

        for &(world_pos, voxel) in changes {
            let chunk_pos = world_pos.div_euclid(chunk_size);
            let local_pos = world_pos.rem_euclid(chunk_size).as_uvec3();
            let overlay = self.chunks.entry(chunk_pos).or_default();

            if overlay.changes.get(&local_pos).copied() == Some(voxel) {
                continue;
            }

            overlay.changes.insert(local_pos, voxel);
            overlay.revision = overlay.revision.wrapping_add(1);
            self.dirty = true;
        }
    }

    /// Returns this chunk's overlay as world-space changes for the common
    /// mutation pipeline. The stable order makes replays and tests deterministic.
    pub fn changes_for_chunk(&self, chunk_pos: IVec3) -> Vec<(IVec3, Voxel)> {
        let Some(overlay) = self.chunks.get(&chunk_pos) else {
            return Vec::new();
        };
        let origin = chunk_pos * TERRAIN_CHUNK_SIZE as i32;
        let mut changes = overlay
            .changes
            .iter()
            .map(|(&local_pos, &voxel)| (origin + local_pos.as_ivec3(), voxel))
            .collect::<Vec<_>>();
        changes.sort_unstable_by_key(|(position, _)| (position.x, position.y, position.z));
        changes
    }

    fn encode(&self) -> Vec<u8> {
        let mut chunks = self.chunks.iter().collect::<Vec<_>>();
        chunks.sort_unstable_by_key(|(position, _)| (position.x, position.y, position.z));

        let mut bytes = Vec::new();
        bytes.extend_from_slice(EDIT_FILE_MAGIC);
        write_u32(&mut bytes, EDIT_FILE_VERSION);
        write_u32(&mut bytes, chunks.len() as u32);

        for (chunk_pos, overlay) in chunks {
            write_i32(&mut bytes, chunk_pos.x);
            write_i32(&mut bytes, chunk_pos.y);
            write_i32(&mut bytes, chunk_pos.z);
            write_u64(&mut bytes, overlay.revision);

            let mut changes = overlay.changes.iter().collect::<Vec<_>>();
            changes.sort_unstable_by_key(|(position, _)| (position.x, position.y, position.z));
            write_u32(&mut bytes, changes.len() as u32);
            for (local_pos, voxel) in changes {
                bytes.push(local_pos.x as u8);
                bytes.push(local_pos.y as u8);
                bytes.push(local_pos.z as u8);
                write_u16(&mut bytes, voxel.id);
            }
        }

        bytes
    }

    fn decode(bytes: &[u8]) -> io::Result<Self> {
        let mut cursor = Cursor::new(bytes);
        let mut magic = [0; 4];
        cursor.read_exact(&mut magic)?;
        if &magic != EDIT_FILE_MAGIC {
            return Err(invalid_data("invalid voxel edit file magic"));
        }

        let version = read_u32(&mut cursor)?;
        if version != EDIT_FILE_VERSION {
            return Err(invalid_data(format!(
                "unsupported voxel edit file version {version}"
            )));
        }

        let chunk_count = read_u32(&mut cursor)?;
        if chunk_count > MAX_SAVED_CHUNKS {
            return Err(invalid_data("voxel edit file contains too many chunks"));
        }

        let mut chunks = HashMap::new();
        for _ in 0..chunk_count {
            let chunk_pos = IVec3::new(
                read_i32(&mut cursor)?,
                read_i32(&mut cursor)?,
                read_i32(&mut cursor)?,
            );
            let revision = read_u64(&mut cursor)?;
            let edit_count = read_u32(&mut cursor)?;
            if edit_count > MAX_EDITS_PER_CHUNK {
                return Err(invalid_data("chunk overlay contains too many edits"));
            }

            let mut changes = HashMap::new();
            for _ in 0..edit_count {
                let mut position = [0; 3];
                cursor.read_exact(&mut position)?;
                if position
                    .iter()
                    .any(|&component| component as u32 >= TERRAIN_CHUNK_SIZE)
                {
                    return Err(invalid_data("chunk overlay contains an invalid position"));
                }
                changes.insert(
                    UVec3::new(position[0] as u32, position[1] as u32, position[2] as u32),
                    Voxel::new(read_u16(&mut cursor)?),
                );
            }
            if chunks
                .insert(chunk_pos, ChunkEditOverlay { revision, changes })
                .is_some()
            {
                return Err(invalid_data("voxel edit file contains a duplicate chunk"));
            }
        }

        if cursor.position() != bytes.len() as u64 {
            return Err(invalid_data("voxel edit file has trailing data"));
        }

        Ok(Self {
            chunks,
            dirty: false,
        })
    }
}

#[derive(Resource, Debug, Clone)]
pub struct VoxelEditPersistence {
    pub path: Option<PathBuf>,
}

impl Default for VoxelEditPersistence {
    fn default() -> Self {
        Self {
            path: Some(PathBuf::from("saves/world/voxel_edits.bin")),
        }
    }
}

pub(crate) fn load_voxel_edits(
    persistence: Res<VoxelEditPersistence>,
    mut store: ResMut<VoxelEditStore>,
) {
    let Some(path) = &persistence.path else {
        return;
    };

    match load_with_backup(path) {
        Ok(Some(loaded)) => {
            info!("Loaded voxel edit overlay from {}", path.display());
            *store = loaded;
        }
        Ok(None) => {}
        Err(error) => warn!(
            "Could not load voxel edit overlay {}: {error}",
            path.display()
        ),
    }
}

pub(crate) fn save_dirty_voxel_edits(
    persistence: Res<VoxelEditPersistence>,
    mut store: ResMut<VoxelEditStore>,
) {
    if !store.dirty {
        return;
    }
    let Some(path) = &persistence.path else {
        return;
    };

    match save_with_backup(path, &store.encode()) {
        Ok(()) => store.dirty = false,
        Err(error) => warn!(
            "Could not save voxel edit overlay {}: {error}",
            path.display()
        ),
    }
}

fn load_with_backup(path: &Path) -> io::Result<Option<VoxelEditStore>> {
    let backup = sidecar_path(path, ".bak");
    let mut last_error = None;

    for candidate in [path.to_path_buf(), backup] {
        if !candidate.exists() {
            continue;
        }
        match fs::read(&candidate).and_then(|bytes| VoxelEditStore::decode(&bytes)) {
            Ok(store) => return Ok(Some(store)),
            Err(error) => last_error = Some(error),
        }
    }

    match last_error {
        Some(error) => Err(error),
        None => Ok(None),
    }
}

fn save_with_backup(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }

    let temporary = sidecar_path(path, ".tmp");
    let backup = sidecar_path(path, ".bak");
    let mut file = File::create(&temporary)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);

    if path.exists() {
        if backup.exists() {
            fs::remove_file(&backup)?;
        }
        fs::rename(path, &backup)?;
    }

    if let Err(error) = fs::rename(&temporary, path) {
        if backup.exists() && !path.exists() {
            let _ = fs::rename(&backup, path);
        }
        return Err(error);
    }

    Ok(())
}

fn sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let mut value = OsString::from(path.as_os_str());
    value.push(suffix);
    PathBuf::from(value)
}

fn invalid_data(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

fn write_u16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn write_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn write_i32(bytes: &mut Vec<u8>, value: i32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn write_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn read_u16(cursor: &mut Cursor<&[u8]>) -> io::Result<u16> {
    let mut bytes = [0; 2];
    cursor.read_exact(&mut bytes)?;
    Ok(u16::from_le_bytes(bytes))
}

fn read_u32(cursor: &mut Cursor<&[u8]>) -> io::Result<u32> {
    let mut bytes = [0; 4];
    cursor.read_exact(&mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_i32(cursor: &mut Cursor<&[u8]>) -> io::Result<i32> {
    let mut bytes = [0; 4];
    cursor.read_exact(&mut bytes)?;
    Ok(i32::from_le_bytes(bytes))
}

fn read_u64(cursor: &mut Cursor<&[u8]>) -> io::Result<u64> {
    let mut bytes = [0; 8];
    cursor.read_exact(&mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voxel_world::{
        core::TerrainChunkData,
        storage::{ChunkMap, VoxelWritePolicy},
    };
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn edit_store_round_trips_deterministically() {
        let mut store = VoxelEditStore::default();
        store.record(&[
            (IVec3::new(-1, 2, 3), Voxel::STONE),
            (IVec3::new(64, 65, 66), Voxel::DIRT),
        ]);

        let bytes = store.encode();
        let decoded = VoxelEditStore::decode(&bytes).unwrap();

        assert_eq!(decoded.encode(), bytes);
        assert_eq!(
            decoded.changes_for_chunk(IVec3::new(-1, 0, 0)),
            vec![(IVec3::new(-1, 2, 3), Voxel::STONE)]
        );
        assert_eq!(
            decoded.changes_for_chunk(IVec3::new(1, 1, 1)),
            vec![(IVec3::new(64, 65, 66), Voxel::DIRT)]
        );
        assert!(!decoded.dirty);
    }

    #[test]
    fn decode_rejects_truncated_data() {
        let mut store = VoxelEditStore::default();
        store.record(&[(IVec3::ZERO, Voxel::STONE)]);
        let mut bytes = store.encode();
        bytes.pop();

        assert!(VoxelEditStore::decode(&bytes).is_err());
    }

    #[test]
    fn edit_overlay_wins_over_generated_features() {
        let position = IVec3::new(4, 5, 6);
        let mut store = VoxelEditStore::default();
        store.record(&[(position, Voxel::EMPTY)]);

        let mut chunks = ChunkMap::default();
        chunks.insert(TerrainChunkData::new_empty(IVec3::ZERO));
        chunks.apply_voxel_changes(
            vec![(position, Voxel::FLOWER_RED)],
            VoxelWritePolicy::ReplaceSoft,
        );
        chunks.apply_voxel_changes(
            store.changes_for_chunk(IVec3::ZERO),
            VoxelWritePolicy::Always,
        );

        assert_eq!(chunks.get_at(position), Some(Voxel::EMPTY));
    }

    #[test]
    fn corrupt_primary_file_falls_back_to_previous_save() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "furaxel-edit-store-{}-{unique}",
            std::process::id()
        ));
        let path = directory.join("voxel_edits.bin");

        let mut previous = VoxelEditStore::default();
        previous.record(&[(IVec3::new(1, 2, 3), Voxel::STONE)]);
        save_with_backup(&path, &previous.encode()).unwrap();

        let mut current = VoxelEditStore::default();
        current.record(&[(IVec3::new(7, 8, 9), Voxel::DIRT)]);
        save_with_backup(&path, &current.encode()).unwrap();
        fs::write(&path, b"corrupt").unwrap();

        let recovered = load_with_backup(&path).unwrap().unwrap();
        assert_eq!(
            recovered.changes_for_chunk(IVec3::ZERO),
            previous.changes_for_chunk(IVec3::ZERO),
        );

        fs::remove_dir_all(directory).unwrap();
    }
}
