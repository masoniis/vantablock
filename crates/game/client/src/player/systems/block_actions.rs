use crate::{
    network::{InboundBlockUpdateMessage, OutboundBreakBlockMessage, OutboundPlaceBlockMessage},
    player::{BreakBlockEvent, PlaceBlockEvent, TargetedBlock},
    world::chunk::ClientChunkMap,
};
use bevy::ecs::prelude::*;
use common::{
    prelude::IVec3,
    world::block::block_registry::{AIR_BLOCK_ID, BlockId},
    world::chunk::{
        CHUNK_SIDE_LENGTH,
        components::{ChunkBlocksComponent, ChunkCoord, ChunkMeshDirty},
    },
};

/// Fires a `BreakBlockEvent` for the currently targeted block and asks the network layer to
/// tell the server.
pub fn break_targeted_block_system(
    targeted_block: Res<TargetedBlock>,
    mut break_block_writer: MessageWriter<BreakBlockEvent>,
    mut outbound_writer: MessageWriter<OutboundBreakBlockMessage>,
) {
    if let Some(block_pos) = targeted_block.position {
        // local event for instant Client-Side Prediction (CSP)
        break_block_writer.write(BreakBlockEvent {
            world_pos: block_pos,
        });

        // networked intent to the server
        outbound_writer.write(OutboundBreakBlockMessage {
            position: block_pos,
        });
    }
}

/// Listens for demultiplexed `ServerMessage::BlockUpdate`s from the server (representing other
/// players' actions or server-side changes) and translates them into local block events.
pub fn handle_incoming_block_updates(
    mut block_updates: MessageReader<InboundBlockUpdateMessage>,
    mut break_block_writer: MessageWriter<BreakBlockEvent>,
    mut place_block_writer: MessageWriter<PlaceBlockEvent>,
) {
    for update in block_updates.read() {
        if update.block_id == AIR_BLOCK_ID {
            break_block_writer.write(BreakBlockEvent {
                world_pos: update.position,
            });
        } else {
            place_block_writer.write(PlaceBlockEvent {
                target_pos: update.position,
                block_id: update.block_id,
            });
        }
    }
}

/// Fires a `PlaceBlockEvent` for the currently targeted block and asks the network layer to
/// tell the server.
pub fn place_targeted_block_system(
    targeted_block: Res<TargetedBlock>,
    mut place_block_writer: MessageWriter<PlaceBlockEvent>,
    mut outbound_writer: MessageWriter<OutboundPlaceBlockMessage>,
) {
    if let (Some(block_pos), Some(normal)) = (targeted_block.position, targeted_block.normal) {
        let target_pos = block_pos + normal;
        let block_id = 1;

        // local event for CSP
        place_block_writer.write(PlaceBlockEvent {
            target_pos,
            block_id,
        });

        // networked intent to the server
        outbound_writer.write(OutboundPlaceBlockMessage {
            position: target_pos,
            block_id,
        });
    }
}

/// A system that handles the `BreakBlockEvent` by mutating local chunk data
/// and marking chunks as dirty for remeshing.
pub fn handle_break_block_events_system(
    // input
    mut events: MessageReader<BreakBlockEvent>,
    chunk_map: Res<ClientChunkMap>,

    // output
    mut blocks_query: Query<&mut ChunkBlocksComponent>,
    mut commands: Commands,
) {
    for event in events.read() {
        apply_block_edit(
            event.world_pos,
            AIR_BLOCK_ID,
            &chunk_map,
            &mut blocks_query,
            &mut commands,
        );
    }
}

/// A system that handles the `PlaceBlockEvent` by mutating local chunk data
/// and marking chunks as dirty for remeshing.
pub fn handle_place_block_events_system(
    // input
    mut events: MessageReader<PlaceBlockEvent>,
    chunk_map: Res<ClientChunkMap>,

    // output
    mut blocks_query: Query<&mut ChunkBlocksComponent>,
    mut commands: Commands,
) {
    for event in events.read() {
        apply_block_edit(
            event.target_pos,
            event.block_id,
            &chunk_map,
            &mut blocks_query,
            &mut commands,
        );
    }
}

/// Writes `block_id` at `world_pos` into the loaded chunk containing it and marks that chunk
/// (plus any neighbor sharing the edited border) for remeshing.
///
/// Chunks are looked up through the `ClientChunkMap`, which only tracks chunk entities.
fn apply_block_edit(
    world_pos: IVec3,
    block_id: BlockId,
    chunk_map: &ClientChunkMap,
    blocks_query: &mut Query<&mut ChunkBlocksComponent>,
    commands: &mut Commands,
) {
    let chunk_pos = ChunkCoord::world_to_chunk_pos(world_pos.as_vec3());

    let Some(entity) = chunk_map.get_entity(chunk_pos) else {
        return;
    };
    let Ok(mut chunk_blocks) = blocks_query.get_mut(entity) else {
        return;
    };

    let local_pos = world_pos - (chunk_pos * CHUNK_SIDE_LENGTH as i32);

    chunk_blocks.get_writer().set_data(
        local_pos.x as usize,
        local_pos.y as usize,
        local_pos.z as usize,
        block_id,
    );

    commands.entity(entity).insert(ChunkMeshDirty);

    // blocks on a chunk border change the neighbor's visible faces and ambient occlusion too
    let max_idx = (CHUNK_SIDE_LENGTH - 1) as i32;
    for axis in [IVec3::X, IVec3::Y, IVec3::Z] {
        let along_axis = local_pos.dot(axis);
        let neighbor_offset = if along_axis == 0 {
            -axis
        } else if along_axis == max_idx {
            axis
        } else {
            continue;
        };

        if let Some(neighbor_entity) = chunk_map.get_entity(chunk_pos + neighbor_offset) {
            commands.entity(neighbor_entity).insert(ChunkMeshDirty);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::chunk::ClientChunkState;
    use bevy::prelude::*;
    use common::world::{block::SOLID_BLOCK_ID, chunk::types::ChunkLod};

    fn spawn_chunk(app: &mut App, pos: IVec3) -> Entity {
        let entity = app
            .world_mut()
            .spawn((
                ChunkCoord { pos },
                ChunkBlocksComponent::new_uniform_solid(ChunkLod(0)),
            ))
            .id();
        app.world_mut()
            .resource_mut::<ClientChunkMap>()
            .chunk_states
            .insert(pos, ClientChunkState::MeshComplete { entity });
        entity
    }

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<ClientChunkMap>();
        app.add_message::<BreakBlockEvent>();
        app.add_systems(Update, handle_break_block_events_system);
        app
    }

    #[test]
    fn breaking_a_border_block_dirties_the_chunk_and_that_neighbor_only() {
        let mut app = app();
        let chunk = spawn_chunk(&mut app, IVec3::ZERO);
        let east = spawn_chunk(&mut app, IVec3::X);
        let north = spawn_chunk(&mut app, IVec3::Z);

        // a camera shares chunk coordinates with chunks but must never be edited
        let camera = app
            .world_mut()
            .spawn(ChunkCoord {
                pos: IVec3::new(5, 0, 5),
            })
            .id();

        let max = CHUNK_SIDE_LENGTH as i32 - 1;
        app.world_mut().write_message(BreakBlockEvent {
            world_pos: IVec3::new(max, 4, 4),
        });
        app.update();

        let world = app.world();
        assert!(world.entity(chunk).contains::<ChunkMeshDirty>());
        assert!(world.entity(east).contains::<ChunkMeshDirty>());
        assert!(!world.entity(north).contains::<ChunkMeshDirty>());
        assert!(!world.entity(camera).contains::<ChunkMeshDirty>());

        let blocks = world.entity(chunk).get::<ChunkBlocksComponent>().unwrap();
        let view = blocks.get_view();
        let common::world::chunk::ChunkView::Dense(data) = view else {
            panic!("edited chunk should be dense");
        };
        assert_eq!(data.get_data(max as usize, 4, 4), AIR_BLOCK_ID);
        assert_eq!(data.get_data(0, 0, 0), SOLID_BLOCK_ID);
    }

    #[test]
    fn edits_outside_loaded_chunks_are_ignored() {
        let mut app = app();
        app.world_mut().write_message(BreakBlockEvent {
            world_pos: IVec3::new(1000, 4, 4),
        });
        app.update();
    }
}
