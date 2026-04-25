use bevy::{ecs::system::RunSystemOnce, prelude::*};
use std::time::{Duration, Instant};
use {
    client::network::{ClientNetworkPlugin, local_connection::setup_client},
    server::network::{ServerNetworkPlugin, systems::start_udp_server},
    shared::network::SharedNetworkPlugin,
};

pub struct UdpClientServerTestEnvironment {
    pub server_app: App,
    pub client_app: App,
}

impl Default for UdpClientServerTestEnvironment {
    fn default() -> Self {
        utils::attach_logger();

        let mut server_app = App::new();
        let mut client_app = App::new();

        // minimal bevy plugins + states
        server_app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
        client_app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));

        // networking setup
        server_app.add_plugins(SharedNetworkPlugin);
        server_app.add_plugins(ServerNetworkPlugin);
        client_app.add_plugins(SharedNetworkPlugin);
        client_app.add_plugins(ClientNetworkPlugin);

        // start udp server
        server_app
            .world_mut()
            .run_system_once(start_udp_server)
            .expect("Failed to run start_udp_server system");

        // setup client
        client_app
            .world_mut()
            .run_system_once(setup_client)
            .expect("Failed to run setup_client system");

        Self {
            server_app,
            client_app,
        }
    }
}

impl UdpClientServerTestEnvironment {
    /// Helper to tick both apps
    pub fn step(&mut self) {
        use std::time::Duration;
        let delta = Duration::from_secs_f32(1.0 / 60.0);

        // manually advance time so lightyear can progress its state machine
        for app in [&mut self.server_app, &mut self.client_app] {
            let mut time = app.world_mut().resource_mut::<Time>();
            time.advance_by(delta);
        }

        self.server_app.update();
        self.client_app.update();
    }

    /// Steps the environment until a condition is met, or panics if it times out.
    pub fn wait_until<F>(&mut self, mut condition: F, timeout: Duration)
    where
        F: FnMut(&mut Self) -> bool,
    {
        let start = Instant::now();

        while !condition(self) {
            if start.elapsed() > timeout {
                panic!("wait_until timed out after {:?}", timeout);
            }

            // step both apps
            self.step();

            // wait in between
            std::thread::sleep(Duration::from_millis(2));
        }
    }
}
