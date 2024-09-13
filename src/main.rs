use std::collections::HashMap;
use std::sync::Arc;
use std::{error::Error, net::SocketAddr};

use bytes::Bytes;
use clap::{command, Parser};
use futures::{SinkExt, StreamExt};
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};
use tokio_util::codec::{BytesCodec, Framed, FramedRead, FramedWrite, LinesCodec};

mod server;
mod client;
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Server or client mode
    #[arg(short, long)]
    server: bool,
    /// Address to connect to
    #[arg(short, long, default_value = "127.0.0.1:8080")]
    address: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    if args.server {
        server::run(args).await?;
    } else {
        client::run(args).await?;
    }
    Ok(())
}


