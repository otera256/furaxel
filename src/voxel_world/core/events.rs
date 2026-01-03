use bevy::prelude::*;

#[derive(Event, Message, Debug, Clone, Copy)]
pub struct ChunkGeneratedEvent(pub IVec3);