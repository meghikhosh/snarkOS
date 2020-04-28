use dpc_node::{
    cli::CLI,
    config::{Config, ConfigCli},
};
use snarkos_dpc::{
    base_dpc::{
        instantiated::{Components, MerkleTreeLedger},
        parameters::PublicParameters,
        DPC,
    },
    dpc::address::AddressPublicKey,
};
use snarkos_dpc_consensus::{miner::MemoryPool, ConsensusParameters};
use snarkos_dpc_network::{
    context::Context,
    protocol::SyncHandler,
    server::{MinerInstance, Server},
};
use snarkos_errors::node::NodeError;
use snarkos_utilities::bytes::FromBytes;
//use snarkos_rpc::start_rpc_server;

use std::{net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;

/// Builds a node from configuration parameters.
/// 1. Creates consensus parameters.
/// 2. Creates new storage database or uses existing.
/// 2. Creates new memory pool or uses existing from storage.
/// 3. Creates network server.
/// 4. Starts rpc server thread.
/// 5. Starts miner thread.
/// 6. Starts network server listener.
async fn start_server(config: Config) -> Result<(), NodeError> {
    if !config.quiet {
        std::env::set_var("RUST_LOG", "info");
        env_logger::init();
    }

    let address = format! {"{}:{}", config.ip, config.port};
    let socket_address = address.parse::<SocketAddr>()?;

    let consensus = ConsensusParameters {
        max_block_size: 1_000_000_000usize,
        max_nonce: u32::max_value(),
        target_block_time: 10i64,
    };

    let mut path = std::env::current_dir()?;
    path.push(&config.path);

    let storage = Arc::new(MerkleTreeLedger::open_at_path(path)?);

    let mut parameters_path = std::env::current_dir()?;
    parameters_path.push("dpc/src/parameters/");

    let parameters = PublicParameters::<Components>::load(&parameters_path)?;

    let memory_pool = MemoryPool::from_storage(&storage.clone())?;
    let memory_pool_lock = Arc::new(Mutex::new(memory_pool.clone()));

    let bootnode = config.bootnodes[0].parse::<SocketAddr>()?;

    let sync_handler = SyncHandler::new(bootnode);
    let sync_handler_lock = Arc::new(Mutex::new(sync_handler));

    let server = Server::new(
        Context::new(
            socket_address,
            config.mempool_interval,
            config.min_peers,
            config.max_peers,
            config.is_bootnode,
            config.bootnodes.clone(),
        ),
        consensus.clone(),
        storage.clone(),
        parameters.clone(),
        memory_pool_lock.clone(),
        sync_handler_lock.clone(),
        10000, // 10 seconds
    );

    // Start rpc thread

    //    if config.jsonrpc {
    //        start_rpc_server(
    //            config.rpc_port,
    //            storage.clone(),
    //            server.context.clone(),
    //            consensus.clone(),
    //            memory_pool_lock.clone(),
    //        )
    //        .await?;
    //    }

    // Start miner thread

    // TODO make this a permanently stored miner address
    //    let rng = &mut thread_rng();
    //    let miner_metadata = [0u8; 32];
    //    let miner_address = DPC::create_address_helper(&parameters.circuit_parameters, &miner_metadata, rng).unwrap();

    let miner_address: AddressPublicKey<Components> = FromBytes::read(&hex::decode(config.coinbase_address)?[..])?;

    if config.miner {
        MinerInstance::new(
            miner_address,
            consensus.clone(),
            parameters,
            storage.clone(),
            memory_pool_lock.clone(),
            server.context.clone(),
        )
        .spawn();
    }

    println!("7");

    // Start server thread

    server.listen().await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), NodeError> {
    let arguments = ConfigCli::new();

    let config: Config = ConfigCli::parse(&arguments)?;

    start_server(config).await
}
