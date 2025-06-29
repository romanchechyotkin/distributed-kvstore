use crate::processor::Processor;

mod proto {
    tonic::include_proto!("proto");
    tonic::include_proto!("service_discovery");
}

use proto::{
    kv_store_server::{KvStore, KvStoreServer},
    service_discovery_client::ServiceDiscoveryClient,
    Address, AppendEntriesRequest, AppendEntriesResponse, Entry, RegisterRequest,
    RequestVoteRequest, RequestVoteResponse,
};

use tokio::{net::TcpListener, sync::Mutex};

use tonic::{
    transport::{Channel, Server as GrpcServer},
    Request, Response, Status,
};

use std::sync::Arc;

pub struct Server {
    tcp_port: u16,
    grpc_port: u16,
    cluster_name: String,

    processor: Arc<Mutex<Processor>>,

    discovery_service_client: Mutex<ServiceDiscoveryClient<Channel>>,

    logs: Vec<Entry>,
}

impl Server {
    pub async fn new(
        tcp_port: u16,
        grpc_port: u16,
        cluster_name: String,
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
            discovery_service_client: Mutex::new(discovery_service_client),
            processor,
            cluster_name,
            logs: vec![],
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
                    port: format!("{}", self.grpc_port),
                }),
                cluster_name: self.cluster_name.clone(),
            })
            .await
            .unwrap();

        println!("register response {:?}", resp);
    }

    async fn run_tcp_server(&self) {
        let addr = format!("127.0.0.1:{}", &self.tcp_port);

        let listener = TcpListener::bind(&addr).await.unwrap();

        println!("tcp listener on {addr}");

        loop {
            let (socket, _) = listener.accept().await.unwrap();
            let processor: Arc<Mutex<Processor>> = Arc::clone(&self.processor);

            tokio::spawn(async move {
                let mut proc = processor.lock().await;
                proc.process_connection(socket).await;
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

        // tokio::spawn(async move {
        //     // get nodes from cluster
        //     // iterate
        //     // send append_entries request
        //     //
        // });
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
