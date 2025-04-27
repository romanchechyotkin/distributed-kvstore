mod proto {
    tonic::include_proto!("proto");
}

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::Mutex,
};

use std::{collections::HashMap, env, sync::Arc};

struct Processor {
    store: Store,
}

impl Processor {
    fn new() -> Processor {
        Processor {
            store: Store::new(),
        }
    }

    async fn process_connection(&mut self, mut conn: TcpStream) {
        let mut request_buffer: [u8; 512] = [0; 512];

        loop {
            let _ = match conn.read(&mut request_buffer).await {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) => {
                    println!("{e}");
                    break;
                }
            };

            let req = String::from_utf8_lossy(&request_buffer)
                .trim_end_matches(char::from(0))
                .to_string();

            println!("got request {req}");

            let elems: Vec<&str> = req.split_whitespace().collect();
            let cmd = elems[0];

            match cmd.to_lowercase().as_str() {
                "ping" => {
                    let pong = "PONG\n";
                    let _ = conn.write(pong.as_bytes()).await.unwrap();
                }
                "set" => {
                    println!("{:?}", elems);
                    if elems.len() < 3 {
                        conn.write("SET <key> <value>\n".as_bytes()).await.unwrap();
                        continue;
                    }

                    let key = elems[1];
                    let value = elems[2];

                    self.store.set(key, value);
                }
                "get" => {
                    println!("{:?}", elems);
                    if elems.len() < 2 {
                        conn.write("GET <key>\n".as_bytes()).await.unwrap();
                        continue;
                    }
                    let key = elems[1];

                    match self.store.get(key) {
                        Some(value) => {
                            conn.write(format!("{value}\n").as_bytes()).await.unwrap();
                        }
                        None => {
                            conn.write("no such key\n".as_bytes()).await.unwrap();
                        }
                    }
                }
                _ => {
                    conn.write("not implemented yet\n".as_bytes())
                        .await
                        .unwrap();
                }
            };

            request_buffer = [0; 512];
        }
    }
}

struct Store {
    store: HashMap<String, String>,
}

impl Store {
    fn new() -> Store {
        Store {
            store: HashMap::new(),
        }
    }

    fn set(&mut self, key: &str, value: &str) -> Option<String> {
        self.store.insert(key.to_string(), value.to_string())
    }

    fn get(&self, key: &str) -> Option<&str> {
        self.store.get(key).map(|s| s.as_str())
    }
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        println!("provide tcp port and grpc port")
    }

    let tcp_port = args[1].parse::<u16>().expect("Invalid TCP port");
    let grpc_port = args[2].parse::<u16>().expect("Invalid gRPC port");

    let tcp_handle = tokio::spawn(async move {
        run_tcp_server(tcp_port).await;
    });

    let grpc_handle = tokio::spawn(async move {
        run_grpc_server(grpc_port).await;
    });

    let _ = tokio::join!(tcp_handle, grpc_handle);
}

async fn run_tcp_server(port: u16) {
    let addr = format!("127.0.0.1:{port}");

    let listener = TcpListener::bind(&addr).await.unwrap();
    let processor = Arc::new(Mutex::new(Processor::new()));

    println!("tcp listener on {addr}");

    loop {
        let (socket, _) = listener.accept().await.unwrap();
        let processor = Arc::clone(&processor);

        tokio::spawn(async move {
            let mut proc = processor.lock().await;
            proc.process_connection(socket).await;
        });
    }
}

async fn run_grpc_server(port: u16) {
    let addr = format!("127.0.0.1:{port}");

    println!("grpc server on {addr}");
}
