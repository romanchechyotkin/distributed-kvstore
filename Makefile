run_discovery_service:
	cargo run --bin=service_discovery 8080

build:
	cargo build

run_master_node:
	cargo run --bin distributed-kvstore 6372 8000 cluster 8080 --master-node

run_slave_node:
	cargo run --bin distributed-kvstore 6373 8001 cluster 8080
