use ::{
    client::network::ClientNetworkPlugins, server::network::ServerNetworkPlugin,
    shared::network::SharedNetworkPlugin,
};
use bevy::prelude::*;
use lightyear::crossbeam::CrossbeamIo;
use lightyear::prelude::client::{Client, Connect};
use lightyear::prelude::server::{Server, Start};
use lightyear::prelude::*;
use std::time::{Duration, Instant};

pub struct CrossbeamClientServerTestEnvironment {
    pub server_app: App,
    pub client_app: App,
}

impl Default for CrossbeamClientServerTestEnvironment {
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

        // client networking setup (ClientNetworkPlugins includes SharedNetworkPlugin)
        client_app.add_plugins(ClientNetworkPlugins);

        // setup crossbeam connection
        setup_crossbeam_connection(&mut server_app, &mut client_app);

        Self {
            server_app,
            client_app,
        }
    }
}

impl CrossbeamClientServerTestEnvironment {
    /// Helper to tick both apps
    pub fn step(&mut self) {
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

/// Links a Server App and a Client App via Crossbeam.
fn setup_crossbeam_connection(server_app: &mut App, client_app: &mut App) -> (Entity, Entity) {
    let (server_io, client_io) = CrossbeamIo::new_pair();

    // setup server
    let server_entity = server_app
        .world_mut()
        .spawn((
            Server::default(),
            Link::default(),
            server_io, // server crossbeam io
        ))
        .id();

    server_app.world_mut().trigger(Start {
        entity: server_entity,
    });

    // setup client
    let client_entity = client_app
        .world_mut()
        .spawn((
            Client::default(),
            Link::default(),
            client_io, // client crossbeam io
        ))
        .id();

    client_app.world_mut().trigger(Connect {
        entity: client_entity,
    });

    (server_entity, client_entity)
}
