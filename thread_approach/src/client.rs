use tokio::net::UnixStream;
use tokio::io::{self, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use serde::{Serialize, Deserialize};
use bincode::serialize;

#[derive(Serialize, Deserialize, Debug)]
struct Data {
    description: String,
    values: Vec<i32>,
    identifier: u32,
}

#[derive(Serialize, Deserialize, Debug)]
enum Message {
    ClientId(String),
    Text(String),
    Numbers(Vec<i32>),
    Exit(String),
    ClientListRequest,
    ClientListResponse(Vec<String>),
    GetDataRequest,
    DataResponse(Data),
}

#[tokio::main]
async fn main() {
    let server_socket_path = "/tmp/rust_uds_example.sock";

    // Connect to the Unix domain socket server
    let socket = UnixStream::connect(server_socket_path).await.unwrap();
    println!("Connected to the server.");

    // Split the stream into read and write halves
    let (mut read_half, mut write_half) = socket.into_split();

    // Send a unique client ID (could be process ID, UUID, or any string)
    let client_id = format!("Client-{}", std::process::id());
    let id_msg = Message::ClientId(client_id.clone());
    let serialized_id = serialize(&id_msg).unwrap();
    write_half.write_all(&serialized_id).await.unwrap();

    // Spawn a task to handle incoming responses from the server
    tokio::spawn(async move {
        let mut buf = vec![0u8; 1024];
        loop {
            let n = read_half.read(&mut buf).await.unwrap();
            if n == 0 {
                println!("Server disconnected");
                return;
            }
            // Deserialize the server's response
            let response: Message = bincode::deserialize(&buf[..n]).unwrap();
            match response {
                Message::ClientListResponse(client_list) => {
                    println!("Connected clients: {:?}", client_list);
                }
                Message::DataResponse(data) => {
                    println!("Received Data from server: {:?}", data);
                }
                _ => {}
            }
        }
    });

    // Create a buffered reader for stdin
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut input = String::new();

    loop {
        println!("Enter 'text', 'numbers', 'connected clients', 'get data', or 'exit':");
        input.clear();
        reader.read_line(&mut input).await.unwrap();
        let mut input = input.trim().to_lowercase();

        let msg = match input.as_str() {
            "text" => {
                println!("Enter your message:");
                input.clear();
                reader.read_line(&mut input).await.unwrap();
                Message::Text(input.trim().to_string())
            }
            "numbers" => {
                println!("Enter a series of comma-separated integers:");
                input.clear();
                reader.read_line(&mut input).await.unwrap();
                let numbers: Vec<i32> = input
                    .trim()
                    .split(',')
                    .map(|num| num.trim().parse().unwrap())
                    .collect();
                Message::Numbers(numbers)
            }
            "connected clients" => {
                Message::ClientListRequest  // Request the list of connected clients
            }
            "get data" => {
                Message::GetDataRequest  // Request the Data struct from the server
            }
            "exit" => {
                Message::Exit(client_id.clone())  // Send the client ID with the Exit message
            }
            _ => {
                println!("Unknown command. Try again.");
                continue;
            }
        };

        // Serialize the message
        let serialized_msg = serialize(&msg).unwrap();

        // Send it over the socket (write_half)
        write_half.write_all(&serialized_msg).await.unwrap();

        // If the message was Exit, break the loop
        if matches!(msg, Message::Exit(_)) {
            break;
        }
    }
}
