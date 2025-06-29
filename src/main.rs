mod proto {
    tonic::include_proto!("proto");
    tonic::include_proto!("service_discovery");
}

mod processor;
mod server;
mod store;

use server::Server;

use clap::Parser;

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

#[tokio::main]
async fn main() {
    let args = Cli::parse();

    let srv = Server::new(
        args.tcp_port,
        args.grpc_port,
        args.cluster_name,
        args.discovery_port,
    );

    _ = srv;
}
