use bevy::prelude::*;
use lightyear::prelude::client::*;
use lightyear::prelude::*;
use std::net::{Ipv4Addr, SocketAddr};

fn setup_client(mut commands: Commands) {
    let server_addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 5000);
    let client_addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 0);

    commands.spawn((Client::default(), Link::default()));
}
