use tokio::net::UnixListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::path::Path;
use std::fs;
use serde::{Serialize, Deserialize};
use bincode::{serialize, deserialize};
use std::sync::{Arc, Mutex};
use std::collections::HashSet;

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
    GetDataRequest,  // Client requests data from the server
    DataResponse(Data),  // Server responds with a Data struct
}

#[tokio::main]
async fn main() {
    let socket_path = "/tmp/rust_uds_example.sock";

    if Path::new(socket_path).exists() {
        fs::remove_file(socket_path).unwrap();
    }

    let listener = UnixListener::bind(socket_path).unwrap();
    println!("Server running and listening on {}", socket_path);

    // Shared list of connected clients
    let clients = Arc::new(Mutex::new(HashSet::new()));

    loop {
        let (mut socket, _addr) = listener.accept().await.unwrap();
        let clients = clients.clone();

        tokio::spawn(async move {
            let mut buf = vec![0u8; 1024];
            let mut client_id = None;

            loop {
                let n = socket.read(&mut buf).await.unwrap();
                if n == 0 {
                    if let Some(id) = &client_id {
                        println!("Client {} disconnected", id);
                        clients.lock().unwrap().remove(id);
                    }
                    return;
                }

                // Deserialize the message
                let msg: Message = deserialize(&buf[..n]).unwrap();
                match msg {
                    Message::ClientId(id) => {
                        println!("Client connected with ID: {}", id);
                        clients.lock().unwrap().insert(id.clone());
                        client_id = Some(id);
                    }
                    Message::Text(text) => {
                        println!("Received Text: {}", text);
                    }
                    Message::Numbers(numbers) => {
                        println!("Received Numbers: {:?}", numbers);
                    }
                    Message::Exit(id) => {
                        println!("Client {} requested exit.", id);
                        clients.lock().unwrap().remove(&id);
                        return;
                    }
                    Message::ClientListRequest => {
                        // Respond with the list of connected clients
                        let client_list: Vec<String> = clients.lock().unwrap().iter().cloned().collect();
                        let response = Message::ClientListResponse(client_list);
                        let serialized_response = serialize(&response).unwrap();
                        socket.write_all(&serialized_response).await.unwrap();
                    }
                    Message::GetDataRequest => {
                        // Create a sample Data struct
                        let data = Data {
                            description: "Example data from the server".to_string(),
                            values: vec![1, 2, 3, 4, 5],
                            identifier: 42,
                        };
                        let response = Message::DataResponse(data);
                        let serialized_response = serialize(&response).unwrap();
                        socket.write_all(&serialized_response).await.unwrap();
                    }
                    _ => {}
                }
            }
        });
    }
}
