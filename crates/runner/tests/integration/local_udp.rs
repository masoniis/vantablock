use crate::common::UdpClientServerTestEnvironment;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::network::{ChatAndSystem, ServerMessage};
use std::time::Duration;

#[test]
#[ignore]
fn sending_a_message() {
    let mut env = UdpClientServerTestEnvironment::default();

    // wait for connection to be established
    env.wait_until(
        |e| {
            let mut query = e.server_app.world_mut().query::<&Connected>();
            query.iter(e.server_app.world()).next().is_some()
        },
        Duration::from_secs(5),
    );

    env.step();

    // ensure client has a receiver before sending server message
    env.client_app
        .world_mut()
        .run_system_once(
            |mut receiver_query: Query<&mut MessageReceiver<ServerMessage>>| {
                receiver_query
                    .single_mut()
                    .expect("Client should have a receiver but it doesn't!");
                info!("Client receiver exists!")
            },
        )
        .unwrap();

    // server send message
    env.server_app
        .world_mut()
        .run_system_once(
            |mut sender_query: Query<(Entity, &mut MessageSender<ServerMessage>)>| {
                for (entity, _) in sender_query.iter() {
                    info!("Server MessageSender entity: {:?}", entity);
                }

                let test_msg = ServerMessage::SyncTime {
                    game_time: 42.0,
                    tick: 0,
                };

                let (entity, mut sender) = sender_query
                    .single_mut()
                    .expect("Server should have exactly one sender but it doesn't!");

                info!(
                    "Sending test message from server via entity {:?}...",
                    entity
                );
                sender.send::<ChatAndSystem>(test_msg.clone());
                info!("Message sent (queued)");
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

            // iter_mut requires passing the world in dynamically
            let mut found = false;
            for mut receiver in query.iter_mut(e.client_app.world_mut()) {
                info_once!("Checking client receiver for messages...");
                for message in receiver.receive() {
                    info_once!("Message received: {:?}", message);
                    if let ServerMessage::SyncTime { game_time, tick } = message {
                        assert_eq!(game_time, 42.0);
                        assert_eq!(tick, 0);
                        found = true;
                    }
                }
            }

            found
        },
        Duration::from_secs(2),
    );
}
