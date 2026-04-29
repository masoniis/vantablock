#[cfg(test)]
mod tests {
    use crate::network::systems::ClientConnection;
    use crate::prelude::*;
    use crate::world::chunk::datagen::gentask_components::NeedsGenerating;
    use crate::world::chunk::manager::ServerChunkManager;
    use crate::world::chunk_loading::{manage_player_chunk_loading_system, ClientChunkTracker};
    use bevy::prelude::*;
    use shared::world::chunk::ChunkCoord;

    #[test]
    fn test_manage_player_chunk_loading() {
        let mut app = App::new();

        // add minimal resources
        app.insert_resource(ServerChunkManager::default());

        // add the system
        app.add_systems(Update, manage_player_chunk_loading_system);

        // spawn a player
        let client_entity = app.world_mut().spawn_empty().id();
        app.world_mut().spawn((
            Transform::from_xyz(0.0, 32.0, 0.0),
            ClientConnection { client_entity },
            ClientChunkTracker::default(),
        ));

        // run once
        app.update();

        // check if chunks were requested
        let chunk_manager = app.world().resource::<ServerChunkManager>();
        assert!(
            !chunk_manager.chunk_states.is_empty(),
            "Chunk states should not be empty after player spawn"
        );

        // verify we have a chunk at (0, 1, 0) - player is at Y=32 which is chunk Y=1
        let player_chunk_coord = IVec3::new(0, 1, 0);
        assert!(chunk_manager.is_chunk_present_or_loading(player_chunk_coord));

        // verify the entity has NeedsGenerating
        let entity = chunk_manager.get_entity(player_chunk_coord).unwrap();
        assert!(app.world().get::<NeedsGenerating>(entity).is_some());
        assert!(app.world().get::<ChunkCoord>(entity).is_some());
    }
}
