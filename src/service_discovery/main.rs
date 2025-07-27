mod proto {
    tonic::include_proto!("service_discovery");
}

use clap::Parser;
use proto::{
    service_discovery_server::{ServiceDiscovery, ServiceDiscoveryServer},
    Address, GetNodesRequest, GetNodesResponse, GetSlavesRequest, GetSlavesResponse,
    RegisterRequest, RegisterResponse,
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use tonic::{transport::Server, Code, Request, Response, Status};

/// Run service discovery
#[derive(Parser)]
struct Cli {
    /// Internal grpc port
    grpc_port: u16,
}

struct DisoveryServer {
    store: Arc<Mutex<HashMap<String, Vec<Address>>>>,
}

impl DisoveryServer {
    fn new() -> DisoveryServer {
        DisoveryServer {
            store: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[tonic::async_trait]
impl ServiceDiscovery for DisoveryServer {
    async fn register(
        &self,
        request: Request<RegisterRequest>,
    ) -> Result<Response<RegisterResponse>, Status> {
        let request = request.into_inner();
        println!("register request {:?}", request);

        let cluster_name = request.cluster_name;

        match request.addr {
            Some(addr) => {
                let mut store = self.store.lock().await;
                match store.get_mut(&cluster_name) {
                    Some(nodes) => {
                        nodes.push(addr);
                    }
                    None => {
                        store.insert(cluster_name.clone(), vec![addr]);
                    }
                }
            }
            None => return Err(Status::new(Code::InvalidArgument, "provide address")),
        }

        let resp = RegisterResponse {};
        Ok(Response::new(resp))
    }

    async fn get_nodes(
        &self,
        request: Request<GetNodesRequest>,
    ) -> Result<Response<GetNodesResponse>, Status> {
        let request = request.into_inner();
        println!("get nodes request {:?}", request);

        let cluster_name = request.cluster_name;
        let store = self.store.lock().await;

        match store.get(&cluster_name) {
            Some(nodes) => {
                let resp = GetNodesResponse {
                    nodes: nodes.clone(),
                };

                Ok(Response::new(resp))
            }
            None => Err(Status::new(Code::InvalidArgument, "no such cluster")),
        }
    }

    async fn get_slaves(
        &self,
        request: Request<GetSlavesRequest>,
    ) -> Result<Response<GetSlavesResponse>, Status> {
        let request = request.into_inner();
        println!("get nodes request {:?}", request);

        let cluster_name = request.cluster_name;
        let store = self.store.lock().await;

        match store.get(&cluster_name) {
            Some(nodes) => {
                let resp = GetSlavesResponse {
                    nodes: nodes.iter().filter(|a| !a.master).cloned().collect(),
                };

                Ok(Response::new(resp))
            }
            None => Err(Status::new(Code::InvalidArgument, "no such cluster")),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("service_discovery");

    let args = Cli::parse();
    let addr = format!("127.0.0.1:{}", args.grpc_port).parse().unwrap();

    let srv = DisoveryServer::new();

    println!("service_discovery server running on grpc {addr}");

    Server::builder()
        .add_service(ServiceDiscoveryServer::new(srv))
        .serve(addr)
        .await?;

    Ok(())
}
