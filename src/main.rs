mod proto {
    tonic::include_proto!("proto");
    tonic::include_proto!("service_discovery");
}

use clap::Parser;

use proto::{
    kv_store_server::{KvStore, KvStoreServer},
    service_discovery_client::ServiceDiscoveryClient,
    Address, AppendEntriesRequest, AppendEntriesResponse, RegisterRequest, RegisterResponse,
    RequestVoteRequest, RequestVoteResponse,
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::Mutex,
};
use tonic::{
    transport::{Channel, Server as GrpcServer},
    Code, Request, Response, Status,
};

use std::{collections::HashMap, sync::Arc};

/// Run kv store
#[derive(Parser)]
struct Cli {
    /// Client tcp port
    tcp_port: u16,

    /// Internal grpc port
    grpc_port: u16,

    /// Cluster name
    cluster_name: String,

    /// Discovery service grpc port
    discovery_port: u16,
}

struct Server {
    server_grpc_port: u16,
    cluster_name: String,

    discovery_service_client: ServiceDiscoveryClient<Channel>,
}

impl Server {
    async fn new(server_port: u16, cluster_name: String, discovery_port: u16) -> Server {
        let client = ServiceDiscoveryClient::connect(format!("http://localhost:{discovery_port}"))
            .await
            .unwrap();

        let mut srv = Server {
            server_grpc_port: server_port,
            discovery_service_client: client,
            cluster_name,
        };

        srv.register_in_service_discovery().await;

        srv
    }

    async fn register_in_service_discovery(&mut self) -> () {
        println!("sent register request {}", self.server_grpc_port);

        let resp = self
            .discovery_service_client
            .register(RegisterRequest {
                addr: Some(Address {
                    host: "127.0.0.1".into(),
                    port: format!("{}", self.server_grpc_port),
                }),
                cluster_name: self.cluster_name.clone(),
            })
            .await
            .unwrap();

        println!("register response {:?}", resp);
    }
}

#[tonic::async_trait]
impl KvStore for Server {
    async fn append_entries(
        &self,
        _request: Request<AppendEntriesRequest>,
    ) -> Result<Response<AppendEntriesResponse>, Status> {
        let resp = AppendEntriesResponse {
            success: true,
            term: String::from(""),
        };
        Ok(Response::new(resp))
    }

    async fn request_vote(
        &self,
        _request: Request<RequestVoteRequest>,
    ) -> Result<Response<RequestVoteResponse>, Status> {
        let resp = RequestVoteResponse {};
        Ok(Response::new(resp))
    }
}

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
    let args = Cli::parse();

    let tcp_handle = tokio::spawn(async move {
        run_tcp_server(args.tcp_port).await;
    });

    let grpc_handle = tokio::spawn(async move {
        run_grpc_server(args.grpc_port, args.cluster_name, args.discovery_port).await;
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

async fn run_grpc_server(port: u16, cluster_name: String, discovery_port: u16) {
    let addr = format!("127.0.0.1:{port}").parse().unwrap();

    println!("grpc server on {addr}");

    let srv = Server::new(port, cluster_name, discovery_port).await;

    let _ = GrpcServer::builder()
        .add_service(KvStoreServer::new(srv))
        .serve(addr)
        .await;
}
