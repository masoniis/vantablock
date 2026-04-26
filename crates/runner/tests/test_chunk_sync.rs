mod common;

use crate::common::networking_test_app::UdpClientServerTestEnvironment;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::network::{ServerMessage, channel::ChatAndSystem};
use std::time::Duration;

#[test]
fn test_pure_connection() {
    let mut env = UdpClientServerTestEnvironment::default();

    // wait for connection to be established
    env.wait_until(
        |e| {
            let mut query = e.server_app.world_mut().query::<&Connected>();
            query.iter(e.server_app.world()).next().is_some()
        },
        Duration::from_secs(5),
    );

    env.client_app.update();
    env.server_app.update();
    env.client_app.update();
    env.server_app.update();

    // server send message
    env.server_app
        .world_mut()
        .run_system_once(
            |mut sender_query: Query<&mut MessageSender<ServerMessage>>| {
                let test_msg = ServerMessage::SyncTime {
                    game_time: 42.0,
                    tick: 100,
                };

                let mut sender = sender_query
                    .single_mut()
                    .expect("Client sender doesn't exist");

                info!("Sending test message from server...");

                sender.send::<ChatAndSystem>(test_msg.clone());
            },
        )
        .unwrap();

    env.step();

    // client receive message
    env.wait_until(
        |e| {
            let mut query = e
                .client_app
                .world_mut()
                .query::<&mut MessageReceiver<ServerMessage>>();
            let mut found = false;

            // iter_mut requires passing the world in dynamically
            for mut receiver in query.iter_mut(e.client_app.world_mut()) {
                for message in receiver.receive() {
                    if let ServerMessage::SyncTime { game_time, tick } = message {
                        assert_eq!(game_time, 42.0);
                        assert_eq!(tick, 100);
                        found = true;
                    }
                }
            }

            found
        },
        Duration::from_secs(2),
    );
}
