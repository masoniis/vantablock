use crate::prelude::*;
use bevy::ecs::{lifecycle::Add, observer::On, system::Commands};
use shared::player::components::NetworkPlayer;

/// Observer that triggers the moment a `NetworkPlayer` component is added to an entity.
/// This typically happens when an entity is replicated from the server.
pub fn dress_predicted_player_observer(trigger: On<Add, NetworkPlayer>, mut _commands: Commands) {
    info!("REPLICATED PLAYER COMING IN {:?}.", trigger.entity);
}
