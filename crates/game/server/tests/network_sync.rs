use bevy::prelude::*;
use common::network::protocol::ServerMessage;
use common::world::chunk::{ChunkBlocksComponent, ChunkCoord, ChunkLod};
use lightyear::prelude::MessageSender;
use server::network::receive::ecs_messages::InboundRequestChunkMessage;
use server::network::types::ClientConnection;
use server::world::chunk::chunk_map::ChunkMap;
use server::world::chunk::components::{ActiveChunk, ModifiedChunk};
use server::world::chunk::loading::{
    ClientChunkTracker, queue_client_chunk_requests_system, sync_chunk_data_to_clients_system,
    unload_distant_chunks_system,
};

/// A server world with one connected player at the origin and one generated chunk.
struct TestServer {
    app: App,
    client_entity: Entity,
    player_entity: Entity,
}

impl TestServer {
    fn new() -> Self {
        let mut app = App::new();
        app.insert_resource(ChunkMap::default());
        app.add_message::<InboundRequestChunkMessage>();
        app.add_systems(
            Update,
            (
                unload_distant_chunks_system,
                queue_client_chunk_requests_system,
                sync_chunk_data_to_clients_system,
            )
                .chain(),
        );

        // spawn server-side client connection and player entities
        let client_entity = app
            .world_mut()
            .spawn(MessageSender::<ServerMessage>::default())
            .id();
        let player_entity = app
            .world_mut()
            .spawn((
                ClientConnection { client_entity },
                ClientChunkTracker::default(),
                common::player::components::LogicalPosition(Vec3::ZERO),
                Transform::from_xyz(0.0, 0.0, 0.0),
            ))
            .id();

        Self {
            app,
            client_entity,
            player_entity,
        }
    }

    fn spawn_generated_chunk(&mut self, coord: IVec3) -> Entity {
        let chunk_blocks = ChunkBlocksComponent::new_uniform_empty(ChunkLod(0));
        let chunk_entity = self
            .app
            .world_mut()
            .spawn((ChunkCoord { pos: coord }, chunk_blocks, ActiveChunk))
            .id();
        self.app
            .world_mut()
            .resource_mut::<ChunkMap>()
            .register_chunk(coord, chunk_entity);
        chunk_entity
    }

    fn is_loaded(&self, coord: IVec3) -> bool {
        self.app
            .world()
            .resource::<ChunkMap>()
            .is_chunk_present(coord)
    }

    fn request(&mut self, coord: IVec3) {
        self.app
            .world_mut()
            .write_message(InboundRequestChunkMessage {
                player: self.client_entity,
                coord: ChunkCoord { pos: coord },
            });
    }

    fn pending(&self) -> &std::collections::HashSet<IVec3> {
        &self
            .app
            .world()
            .get::<ClientChunkTracker>(self.player_entity)
            .unwrap()
            .pending
    }
}

#[test]
fn test_requested_chunk_is_sent() {
    let mut server = TestServer::new();
    let coord = IVec3::new(0, 0, 0);
    server.spawn_generated_chunk(coord);

    server.request(coord);
    server.app.update();

    // the chunk was generated, so it was sent and is no longer pending
    assert!(
        server.pending().is_empty(),
        "requested chunk should be sent"
    );
}

#[test]
fn test_requested_chunk_waits_for_generation() {
    let mut server = TestServer::new();
    let coord = IVec3::new(1, 0, 0);

    server.request(coord);
    server.app.update();
    assert!(
        server.pending().contains(&coord),
        "ungenerated chunk should stay pending"
    );

    server.spawn_generated_chunk(coord);
    server.app.update();
    assert!(
        server.pending().is_empty(),
        "chunk should be sent once generated"
    );
}

/// Regression: the server used to remember every chunk it ever sent, so a chunk the client
/// unloaded and needed again was never resent, leaving permanent holes in the world.
#[test]
fn test_rerequested_chunk_is_sent_again() {
    let mut server = TestServer::new();
    let coord = IVec3::new(0, 0, 0);
    server.spawn_generated_chunk(coord);

    server.request(coord);
    server.app.update();
    assert!(server.pending().is_empty());

    // the client unloaded the chunk and walked back, so it asks again
    server.request(coord);
    server.app.update();
    assert!(
        server.pending().is_empty(),
        "re-requested chunk should be sent again"
    );
}

#[test]
fn test_out_of_range_requests_are_ignored() {
    let mut server = TestServer::new();
    let far = IVec3::new(1000, 0, 0);

    server.request(far);
    server.app.update();

    assert!(
        server.pending().is_empty(),
        "far away request should be ignored"
    );
}

/// Regression: the server never unloaded chunks, so memory grew without bound while exploring.
#[test]
fn test_distant_chunks_are_unloaded() {
    let mut server = TestServer::new();
    let near = IVec3::new(0, 0, 0);
    let far = IVec3::new(100, 0, 0);
    server.spawn_generated_chunk(near);
    let far_entity = server.spawn_generated_chunk(far);

    server.app.update();

    assert!(
        server.is_loaded(near),
        "chunk near the player should stay loaded"
    );
    assert!(
        !server.is_loaded(far),
        "chunk far from every player should unload"
    );
    assert!(
        server.app.world().get_entity(far_entity).is_err(),
        "unloaded chunk entity should be despawned"
    );
}

#[test]
fn test_modified_distant_chunks_stay_loaded() {
    let mut server = TestServer::new();
    let far = IVec3::new(100, 0, 0);
    let far_entity = server.spawn_generated_chunk(far);
    server
        .app
        .world_mut()
        .entity_mut(far_entity)
        .insert(ModifiedChunk);

    server.app.update();

    assert!(
        server.is_loaded(far),
        "edited chunks must stay loaded until chunks can be saved"
    );
}
