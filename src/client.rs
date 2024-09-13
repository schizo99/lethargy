use crate::Args;
use std::{error::Error, net::SocketAddr};
use futures::{SinkExt, StreamExt};
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};
use tokio_util::codec::{BytesCodec, Framed, FramedRead, FramedWrite, LinesCodec};

pub async fn run(args: Args) -> Result<(), Box<dyn Error>> {
    // let stdin = FramedRead::new(io::stdin(), BytesCodec::new());
    // let stdin = stdin.map(|i| i.map(|bytes| bytes.freeze()));
    // let stdout = FramedWrite::new(io::stdout(), BytesCodec::new());
    let addr = &args
        .address
        .parse::<SocketAddr>()
        .expect("failed to parse address");
    println!("Connecting to server: {}", addr);
    //tcp::connect(&addr).await.expect("failed to connect to server");
    let mut stream = TcpStream::connect(args.address).await.expect("Failed to connect to server");
    println!("Connected to the server!");
    // Send a message to the server
    let msg = b"schizo\n";
    stream.write_all(msg).await.expect("Failed to write to socket");
    println!("Sent message: {:?}", String::from_utf8_lossy(msg));
    // Read the server's response
    // let mut buffer = [0; 512];
    // let bytes_read = stream.read(&mut buffer).await.expect("Failed to read from socket");
    // println!("Received reply: {}", String::from_utf8_lossy(&buffer[..bytes_read]));

    let mut lines = Framed::new(stream, LinesCodec::new());
    while let Some(line) = lines.next().await {
        match line {
            Ok(line) => {
                println!("{}", line);
            }
            Err(e) => {
                println!("an error occurred; error = {:?}", e);
            }
        }
    }
    Ok(())

}
