use crate::processor::Processor;

mod proto {
    tonic::include_proto!("proto");
    tonic::include_proto!("service_discovery");
}

use proto::{
    kv_store_server::{KvStore, KvStoreServer},
    service_discovery_client::ServiceDiscoveryClient,
    Address, AppendEntriesRequest, AppendEntriesResponse, GetSlavesRequest, RegisterRequest,
    RequestVoteRequest, RequestVoteResponse,
};

use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
    sync::Mutex,
};

use tonic::{
    transport::{Channel, Server as GrpcServer},
    Request, Response, Status,
};

use std::sync::Arc;

pub struct Server {
    tcp_port: u16,
    grpc_port: u16,
    cluster_name: String,
    master: bool,

    processor: Arc<Mutex<Processor>>,

    discovery_service_client: Mutex<ServiceDiscoveryClient<Channel>>,
}

impl Server {
    pub async fn new(
        tcp_port: u16,
        grpc_port: u16,
        cluster_name: String,
        master: bool,
        discovery_port: u16,
    ) -> Arc<Server> {
        let discovery_service_client =
            ServiceDiscoveryClient::connect(format!("http://localhost:{discovery_port}"))
                .await
                .unwrap();

        let processor: Arc<Mutex<Processor>> = Arc::new(Mutex::new(Processor::new()));

        let srv = Arc::new(Server {
            tcp_port,
            grpc_port,
            master,
            discovery_service_client: Mutex::new(discovery_service_client),
            processor,
            cluster_name,
        });

        let srv_grpc = Arc::clone(&srv);
        srv_grpc.register_in_service_discovery().await;

        let srv_tcp = Arc::clone(&srv);

        let tcp_handle = tokio::spawn(async move {
            srv_tcp.run_tcp_server().await;
        });

        let grpc_handle = tokio::spawn(async move {
            srv_grpc.run_grpc_server().await;
        });

        let _ = tokio::join!(tcp_handle, grpc_handle);

        srv
    }

    async fn register_in_service_discovery(&self) -> () {
        println!("sent register request {}", self.grpc_port);

        let mut client = self.discovery_service_client.lock().await;

        let resp = client
            .register(RegisterRequest {
                addr: Some(Address {
                    host: "127.0.0.1".into(),
                    grpc_port: format!("{}", self.grpc_port),
                    tcp_port: format!("{}", self.tcp_port),
                    master: self.master,
                }),
                cluster_name: self.cluster_name.clone(),
            })
            .await
            .unwrap();

        println!("register response {:?}", resp);

        if self.master {
            let resp = client
                .get_slaves(GetSlavesRequest {
                    cluster_name: self.cluster_name.clone(),
                })
                .await
                .unwrap();

            println!("get slaves nodes response {:?}", resp);
        }
    }

    async fn run_tcp_server(self: Arc<Self>) {
        let addr = format!("127.0.0.1:{}", &self.tcp_port);

        let listener = TcpListener::bind(&addr).await.unwrap();

        println!("tcp listener on {addr}");

        loop {
            let (socket, _) = listener.accept().await.unwrap();

            let srv: Arc<Server> = Arc::clone(&self);

            tokio::spawn(async move {
                srv.process_tcp_connection(socket).await;
            });
        }
    }

    async fn run_grpc_server(self: Arc<Self>) {
        let addr = format!("127.0.0.1:{}", &self.grpc_port).parse().unwrap();

        println!("grpc server on {addr}");

        let _ = GrpcServer::builder()
            .add_service(KvStoreServer::new(self))
            .serve(addr)
            .await;
    }

    async fn process_tcp_connection(self: Arc<Self>, conn: TcpStream) {
        let processor: Arc<Mutex<Processor>> = Arc::clone(&self.processor);
        let mut proc = processor.lock().await;

        match proc.process_connection(conn).await {
            Some(cmd) => {
                let mut client = self.discovery_service_client.lock().await;

                let resp = client
                    .get_slaves(GetSlavesRequest {
                        cluster_name: self.cluster_name.clone(),
                    })
                    .await
                    .unwrap();

                println!("get slaves response {:?}", resp);

                let addresses: Vec<Address> = resp.get_ref().nodes.to_vec();
                let mut slave_addr: String = String::from("");
                let mut set_cmd: String = String::from("");

                for addr in &addresses {
                    slave_addr = format!("{}:{}", addr.host, addr.tcp_port);
                    println!("result addr {slave_addr}");

                    let mut stream = TcpStream::connect(slave_addr).await.unwrap();

                    set_cmd = format!("SET {} {}", cmd.0, cmd.1);
                    println!("result set_cmd {set_cmd}");

                    let _ = stream.write_all(set_cmd.as_bytes()).await;
                    let _ = stream.flush().await;
                }
            }
            None => println!("nothing"),
        }

        println!("done");
    }
}

#[tonic::async_trait]
impl KvStore for Arc<Server> {
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
