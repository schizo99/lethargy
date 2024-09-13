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

mod tcp {
    use bytes::Bytes;
    use core::str;
    use futures::{SinkExt, StreamExt};
    use std::{error::Error, io, net::SocketAddr};
    use tokio::net::TcpStream;
    use tokio_util::codec::{BytesCodec, Framed, FramedRead, FramedWrite, LinesCodec};

    pub async fn connect(addr: &SocketAddr) -> Result<(), Box<dyn Error>> {
        // let mut stream = TcpStream::connect(addr).await?;
        // let (r, w) = stream.split();
        // let mut sink = FramedWrite::new(w, BytesCodec::new());
        // // filter map Result<BytesMut, Error> stream into just a Bytes stream to match stdout Sink
        // // on the event of an Error, log the error and end the stream
        // let mut framed_stream = FramedRead::new(r, BytesCodec::new());
        let conn = TcpStream::connect(addr).await?;

        let mut server = Framed::new(conn, LinesCodec::new_with_max_length(1024));

        while let Some(Ok(line)) = server.next().await {
            println!("{}", line);
        }
        // Process each incoming message sequentially
        // while let Some(result) = framed_stream.next().await {
        //     match result {
        //         Ok(bytes) => {
        //             let string = str::from_utf8(&bytes).unwrap();
        //             if string.contains("<EXIT>") {
        //                 println!("Client has disconnected {}", &b"<EXIT>"[0]);
        //                 break; // Exit loop when <EXIT> is received
        //             }

        //             // Process the message
        //             println!("{}", String::from_utf8_lossy(&bytes));

        //             // Send acknowledgment (ACK) back to the server after processing the message
        //             sink.send(Bytes::from("<ACK>"))
        //                 .await
        //                 .expect("failed to send ACK");
        //         }
        //         Err(e) => {
        //             println!("Failed to read from socket; error={}", e);
        //             break;
        //         }
        //     }
        //     sink.send(Bytes::from("Va vill du?"))
        //         .await
        //         .expect("failed to send ACK");
        // }
        Ok(())
    }
}

async fn client(args: Args) {
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

}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    if args.server {
        let listener = TcpListener::bind(&args.address).await?;
        let state = Arc::new(Mutex::new(Shared::new()));
        println!("server running on {}", &args.address);

        loop {
            // Asynchronously wait for an inbound TcpStream.
            let (stream, addr) = listener.accept().await?;

            // Clone a handle to the `Shared` state for the new connection.
            let state = Arc::clone(&state);

            // Spawn our handler to be run asynchronously.
            tokio::spawn(async move {
                println!("accepted connection");
                if let Err(e) = process(state, stream, addr).await {
                    println!("an error occurred; error = {:?}", e);
                }
            });
        }
    } else {
        client(args).await;
    }
    Ok(())
}

/// Shorthand for the transmit half of the message channel.
type Tx = mpsc::UnboundedSender<String>;

/// Shorthand for the receive half of the message channel.
type Rx = mpsc::UnboundedReceiver<String>;

/// Data that is shared between all peers in the chat server.
///
/// This is the set of `Tx` handles for all connected clients. Whenever a
/// message is received from a client, it is broadcasted to all peers by
/// iterating over the `peers` entries and sending a copy of the message on each
/// `Tx`.
struct Shared {
    peers: HashMap<SocketAddr, Tx>,
    username: Vec<String>,
}

/// The state for each connected client.
struct Peer {
    /// The TCP socket wrapped with the `Lines` codec, defined below.
    ///
    /// This handles sending and receiving data on the socket. When using
    /// `Lines`, we can work at the line level instead of having to manage the
    /// raw byte operations.
    lines: Framed<TcpStream, LinesCodec>,

    /// Receive half of the message channel.
    ///
    /// This is used to receive messages from peers. When a message is received
    /// off of this `Rx`, it will be written to the socket.
    rx: Rx,
    username: String,
}

impl Shared {
    /// Create a new, empty, instance of `Shared`.
    fn new() -> Self {
        Shared {
            peers: HashMap::new(),
            username: vec![],
        }
    }

    /// Send a `LineCodec` encoded message to every peer, except
    /// for the sender.
    async fn broadcast(&mut self, sender: SocketAddr, message: &str) {
        for peer in self.peers.iter_mut() {
            if *peer.0 != sender {
                let _ = peer.1.send(message.into());
            }
        }
    }
}

impl Peer {
    /// Create a new instance of `Peer`.
    async fn new(
        state: Arc<Mutex<Shared>>,
        lines: Framed<TcpStream, LinesCodec>,
        username: String,
    ) -> io::Result<Peer> {
        // Get the client socket address
        let addr = lines.get_ref().peer_addr()?;

        // Create a channel for this peer
        let (tx, rx) = mpsc::unbounded_channel();

        // Add an entry for this `Peer` in the shared state map.
        state.lock().await.peers.insert(addr, tx);
        state.lock().await.username.push(username.clone());

        Ok(Peer { lines, rx, username })
    }
}

fn users() -> Vec<String> {
    let userdb = sqlite::open("dgamelaunch.db").unwrap();
        let query = "SELECT username FROM dglusers";
        let mut users = vec![];
        userdb
            .iterate(query, |pairs| {
                for &(_name, value) in pairs.iter() {
                    match value {
                        None => println!("{}: NULL", _name),
                        Some(s) => users.push(s.to_string()),
                    }
                }
                true
            })
            .unwrap();
    users
}

/// Process an individual chat client
async fn process(
    state: Arc<Mutex<Shared>>,
    stream: TcpStream,
    addr: SocketAddr,
) -> Result<(), Box<dyn Error>> {
    let users = users();
    let mut lines = Framed::new(stream, LinesCodec::new());

    // Read the first line from the `LineCodec` stream to get the username.
    let username = match lines.next().await {
        Some(Ok(line)) => line,
        // We didn't get a line so we return early here.
        _ => {
            println!("Failed to get username from {}. Client disconnected.", addr);
            return Ok(());
        }
    };

    // Register our peer with state which internally sets up some channels.
    let mut peer = Peer::new(state.clone(), lines, username).await?;
    peer.lines.send(&format!("Welcome, {}!", peer.username)).await?;
    peer.lines.send(&format!("<users>{:?}", users)).await?;
    for user in state.lock().await.username.iter() {
        peer.lines.send(&format!("{} is online", user)).await?;
    }
    // A client has connected, let's let everyone know.
    {
        let mut state = state.lock().await;
        let msg = format!("{} has joined the chat", peer.username);
        println!("{}", msg);
        state.broadcast(addr, &msg).await;
    }

    // Process incoming messages until our stream is exhausted by a disconnect.
    loop {
        tokio::select! {
            // A message was received from a peer. Send it to the current user.
            Some(msg) = peer.rx.recv() => {
                peer.lines.send(&msg).await?;
            }
            result = peer.lines.next() => match result {
                // A message was received from the current user, we should
                // broadcast this message to the other users.
                Some(Ok(msg)) => {
                    let mut state = state.lock().await;
                    let msg = format!("{}: {}", peer.username, msg);

                    state.broadcast(addr, &msg).await;
                }
                // An error occurred.
                Some(Err(e)) => {
                    println!(
                        "an error occurred while processing messages for {}; error = {:?}",
                        peer.username,
                        e
                    );
                }
                // The stream has been exhausted.
                None => break,
            },
        }
    }

    // If this section is reached it means that the client was disconnected!
    // Let's let everyone still connected know about it.
    {
        let mut state = state.lock().await;
        state.peers.remove(&addr);
        state.username.retain(|x| x != &peer.username);

        let msg = format!("{} has left the chat", peer.username);
        println!("{}", msg);
        state.broadcast(addr, &msg).await;
    }

    Ok(())
}
