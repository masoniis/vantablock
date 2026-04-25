mod common;

use crate::common::networking_test_app::TestEnvironment;
use bevy::ecs::system::RunSystemOnce; // <--- Critical for running inline systems in tests!
use bevy::prelude::*;
use lightyear::prelude::*; // Assuming this brings in MessageEvent
use shared::network::protocol::server::ServerMessage;
use std::time::Duration;

#[test]
fn test_pure_connection() {
    utils::attach_logger();

    let mut env = TestEnvironment::default();

    // wait for connection to be established
    let mut connected = false;
    for _ in 0..200 {
        env.step();
        std::thread::sleep(std::time::Duration::from_millis(10));

        let mut query = env.server_app.world_mut().query::<&Connected>();
        if query.iter(env.server_app.world()).next().is_some() {
            connected = true;
            break;
        }
    }

    assert!(
        connected,
        "Handshake timed out - client never connected to server"
    );

    // // 2. SERVER SENDS MESSAGE
    // env.server_app
    //     .world_mut()
    //     .run_system_once(
    //         // Replace `ResMut<ConnectionManager>` with whatever Resource you use
    //         // in your actual chunk syncing systems to send messages!
    //         |mut server_connection: ResMut<ServerConnectionManager>| {
    //             let test_msg = ServerMessage::SyncTime {
    //                 game_time: 42.0,
    //                 tick: 100,
    //             };
    //
    //             // Send the message exactly as you do in your real game code
    //             // e.g., server_connection.send_message_to_target::<Channel, _>(test_msg, NetworkTarget::All).unwrap();
    //         },
    //     )
    //     .unwrap();
    //
    // // 3. TICK NETWORK
    // for _ in 0..10 {
    //     env.step();
    //     std::thread::sleep(Duration::from_millis(10));
    // }
    //
    // // 4. CLIENT RECEIVES MESSAGE
    // // (Your client receiving code was completely correct, leave it exactly as it was!)
    // env.client_app
    //     .world_mut()
    //     .run_system_once(|mut query: Query<&mut MessageReceiver<ServerMessage>>| {
    //         let mut message_received = false;
    //
    //         for mut receiver in query.iter_mut() {
    //             for message in receiver.receive() {
    //                 if let ServerMessage::SyncTime { game_time, tick } = message {
    //                     assert_eq!(game_time, 42.0);
    //                     assert_eq!(tick, 100);
    //                     message_received = true;
    //                 }
    //             }
    //         }
    //
    //         assert!(message_received, "Client never received the ServerMessage!");
    //     })
    //     .unwrap();
}
