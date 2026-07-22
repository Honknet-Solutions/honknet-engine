use anyhow::Result;
use clap::Parser;
use honknet_math::Vec2;
use honknet_net_core::{
    encode_message,
    Channel,
    NetworkMessage
};
use honknet_net_transport::{
    NetworkTransport,
    UdpTransport
};
use serde::{
    Deserialize,
    Serialize
};
use std::{
    net::SocketAddr,
    time::Duration
};
#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "127.0.0.1:3015")]
    server: SocketAddr,
    #[arg(long, default_value_t = 600)]
    ticks: u32
}

#[derive(Serialize, Deserialize)]
struct Hello {
    client_version: String
}

impl NetworkMessage for Hello {
    const ID: u16 = 1;
}

#[derive(Serialize, Deserialize)]
struct Input {
    sequence: u32,
    movement: Vec2
}

impl NetworkMessage for Input {
    const ID: u16 = 2;
}

#[tokio::main]
async fn main() -> Result<()> {
    let a = Args::parse();
    let t = UdpTransport::bind("0.0.0.0:0".parse()?).await?;
    let p = t.connect(a.server).await;
    let(_, h, _) = encode_message(&Hello {
        client_version: "1.0.0-rc.1".into()
    }, false)?;
    t.send(p, Channel::Control, Hello::ID, &h).await?;
    for seq in 0..a.ticks {
        let movement = Vec2::new((seq as f32 * 0.04).cos(), (seq as f32 * 0.04).sin());
        let(_, b, _) = encode_message(&Input {
            sequence: seq, movement
        }, false)?;
        t.send(p, Channel::UnreliableSequenced, Input::ID, &b).await?;
        let _ = t.poll().await?;
        tokio::time::sleep(Duration::from_millis(33)).await;
    }
    t.disconnect(p, "test complete").await?;
    Ok(())
}
