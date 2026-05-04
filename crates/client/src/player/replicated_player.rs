use crate::prelude::*;
use bevy::ecs::{lifecycle::Add, observer::On, system::Commands};
use shared::player::components::NetworkPlayer;

/// Observer that triggers the moment a `Player` component is added to an entity.
pub fn dress_predicted_player_observer(trigger: On<Add, NetworkPlayer>, mut _commands: Commands) {
    info!("REPLICATED PLAYER COMING IN {}", trigger.entity);
}
