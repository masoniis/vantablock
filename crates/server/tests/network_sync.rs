use bevy::prelude::*;
use lightyear::prelude::MessageSender;
use server::network::systems::ClientConnection;
use server::world::chunk::manager::ServerChunkManager;
use server::world::chunk_loading::{ClientChunkTracker, sync_chunk_data_to_clients_system};
use shared::network::protocol::server::ServerMessage;
use shared::world::chunk::{ChunkBlocksComponent, ChunkCoord, ChunkLod};

#[test]
fn test_chunk_data_sync_to_client() {
    let mut app = App::new();

    // add resources
    app.insert_resource(ServerChunkManager::default());

    // spawn server-side player entity
    let client_entity = app
        .world_mut()
        .spawn(MessageSender::<ServerMessage>::default())
        .id();
    let player_entity = app
        .world_mut()
        .spawn((
            ClientConnection { client_entity },
            ClientChunkTracker::default(),
            Transform::from_xyz(0.0, 0.0, 0.0),
        ))
        .id();

    // spawn a generated chunk
    let coord = IVec3::new(0, 0, 0);
    let chunk_blocks = ChunkBlocksComponent::new_uniform_empty(ChunkLod(0));
    let chunk_entity = app
        .world_mut()
        .spawn((ChunkCoord { pos: coord }, chunk_blocks))
        .id();

    // mark it as active in the manager
    app.world_mut()
        .resource_mut::<ServerChunkManager>()
        .mark_as_active(coord, chunk_entity);

    // run the sync system
    // we need to use a manual system call or add it to a schedule
    let mut schedule = Schedule::default();
    schedule.add_systems(sync_chunk_data_to_clients_system);
    schedule.run(app.world_mut());

    // verify that the tracker was updated
    let tracker = app
        .world()
        .get::<ClientChunkTracker>(player_entity)
        .unwrap();
    assert!(
        tracker.sent_chunks.contains(&coord),
        "Chunk should be marked as sent in tracker"
    );

    // verify that a message was "sent" (MessageSender is just a component we can check if it has messages,
    // but Lightyear's MessageSender doesn't store them in a way that's easy to check without full setup)
}
