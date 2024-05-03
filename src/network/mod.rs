use std::{
    cmp::Ordering,
    collections::HashMap,
    io::Write,
    sync::{Arc, RwLock},
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

use serde_derive::{Deserialize, Serialize};

use crate::{blockchain, Error, Result};

const CENTRAL: &str = "127.0.0.1:3000";
const VERSION: u32 = 1;

struct Network {
    node_address: String,
    mine_address: String,
    known_nodes: Vec<String>,
    blocks_in_transit: Vec<Vec<u8>>,
    memory_pool: HashMap<String, blockchain::Transaction>,
}

#[derive(Serialize, Deserialize)]
enum Command {
    Block(Block),
    GetBlocks(GetBlocks),
    GetData(GetData),
    Inv(Inv),
    Transaction(Transaction),
    Version(Version),
}

#[derive(Serialize, Deserialize)]
struct Block {
    addr_from: String,
    block: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
struct GetBlocks {
    addr_from: String,
}

#[derive(Serialize, Deserialize)]
struct GetData {
    addr_from: String,
    data_type: String,
    id: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
struct Inv {
    addr_from: String,
    inv_type: String,
    items: Vec<Vec<u8>>,
}

#[derive(Serialize, Deserialize)]
struct Transaction {
    addr_from: String,
    tx: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
struct Version {
    version: u32,
    best_height: u64,
    addr_from: String,
}

async fn send_block(
    network: Arc<RwLock<Network>>,
    address: &str,
    block: &blockchain::Block,
) -> Result<()> {
    println!("send_block");
    let node_address = { network.read().unwrap().node_address.clone() };

    let block = Block {
        addr_from: node_address,
        block: bincode::serialize(block)?,
    };
    let request = bincode::serialize(&Command::Block(block))?;

    send_data(network, address, &request).await?;
    Ok(())
}

async fn send_get_blocks(network: Arc<RwLock<Network>>, address: &str) -> Result<()> {
    println!("send_get_blocks");
    let node_address = { network.read().unwrap().node_address.clone() };

    let get_blocks = GetBlocks {
        addr_from: node_address,
    };

    let request = bincode::serialize(&Command::GetBlocks(get_blocks))?;

    send_data(network, address, &request).await?;
    Ok(())
}

async fn send_get_data(
    network: Arc<RwLock<Network>>,
    address: &str,
    data_type: String,
    id: Vec<u8>,
) -> Result<()> {
    println!("send_get_data");
    let node_address = { network.read().unwrap().node_address.clone() };

    let get_data = GetData {
        addr_from: node_address,
        data_type,
        id,
    };

    let request = bincode::serialize(&Command::GetData(get_data))?;

    send_data(network, address, &request).await?;
    Ok(())
}

async fn send_inv(
    network: Arc<RwLock<Network>>,
    address: &str,
    inv_type: String,
    items: Vec<Vec<u8>>,
) -> Result<()> {
    println!("send_inv");
    let node_address = { network.read().unwrap().node_address.clone() };
    let inv = Inv {
        addr_from: node_address,
        inv_type,
        items,
    };

    let request = bincode::serialize(&Command::Inv(inv))?;
    send_data(network, address, &request).await?;
    Ok(())
}

async fn send_transaction(
    network: Arc<RwLock<Network>>,
    address: &str,
    tx: &blockchain::Transaction,
) -> Result<()> {
    println!("send_transaction");
    let node_address = { network.read().unwrap().node_address.clone() };

    let transaction = Transaction {
        addr_from: node_address,
        tx: bincode::serialize(tx)?,
    };
    let request = bincode::serialize(&Command::Transaction(transaction))?;

    send_data(network, address, &request).await?;
    Ok(())
}

async fn send_version(
    network: Arc<RwLock<Network>>,
    address: &str,
    chain: Arc<RwLock<blockchain::BlockChain>>,
) -> Result<()> {
    println!("send_version");
    let node_address = { network.read().unwrap().node_address.clone() };
    let best_height = { chain.read().unwrap().get_best_height()? };

    let version = Version {
        version: VERSION,
        addr_from: node_address,
        best_height,
    };
    let request = bincode::serialize(&Command::Version(version))?;

    send_data(network, address, &request).await?;

    Ok(())
}

async fn send_data(network: Arc<RwLock<Network>>, addr: &str, request: &[u8]) -> Result<()> {
    let mut socket = match TcpStream::connect(addr).await {
        Ok(socket) => socket,
        Err(err) => {
            {
                network
                    .write()
                    .unwrap()
                    .known_nodes
                    .retain(|node| *node != addr);
            }
            return Err(Error::CustomError(format!(
                "Node {} is not available: {}",
                addr, err
            )));
        }
    };

    socket.write_all(request).await?;

    Ok(())
}

async fn handle_block(
    network: Arc<RwLock<Network>>,
    block: Block,
    chain: Arc<RwLock<blockchain::BlockChain>>,
) -> Result<()> {
    println!("handle_block");
    let b: blockchain::Block = bincode::deserialize(&block.block)?;

    {
        chain.write().unwrap().add_block(&b)?;
    }

    println!("Added block {:?}", hex::encode(b.hash));

    let blocks_in_transit = { network.read().unwrap().blocks_in_transit.clone() };

    if !blocks_in_transit.is_empty() {
        let block_hash = blocks_in_transit[0].clone();
        send_get_data(
            network.clone(),
            &block.addr_from,
            "block".to_owned(),
            block_hash,
        )
        .await?;

        {
            network.write().unwrap().blocks_in_transit.pop();
        }
    } else {
        let chain_clone = { chain.read().unwrap().clone() };
        let utxo_set = blockchain::UTXOSet::new(chain_clone);
        utxo_set.reindex()?;
    }

    Ok(())
}

async fn handle_get_blocks(
    network: Arc<RwLock<Network>>,
    get_block: GetBlocks,
    chain: Arc<RwLock<blockchain::BlockChain>>,
) -> Result<()> {
    println!("handle_get_blocks");
    let block_hashes = { chain.read().unwrap().get_block_hashes()? };

    send_inv(
        network,
        &get_block.addr_from,
        "block".to_owned(),
        block_hashes,
    )
    .await?;

    Ok(())
}

async fn handle_get_data(
    network: Arc<RwLock<Network>>,
    get_data: GetData,
    chain: Arc<RwLock<blockchain::BlockChain>>,
) -> Result<()> {
    println!("handle_get_data");

    if get_data.data_type == *"block" {
        let b = { chain.read().unwrap().get_block(&get_data.id)? };

        if b.is_none() {
            return Err(Error::CustomError(format!(
                "Block {:?} doesn't existed!",
                &get_data.id
            )));
        } else {
            send_block(network.clone(), &get_data.addr_from, &b.unwrap()).await?;
        }
    }

    if get_data.data_type == *"tx" {
        let tx_id = hex::encode(get_data.id);
        let tx = {
            network
                .read()
                .unwrap()
                .memory_pool
                .get(&tx_id)
                .unwrap()
                .clone()
        };

        send_transaction(network, &get_data.addr_from, &tx).await?;
    }

    Ok(())
}

async fn handle_inv(network: Arc<RwLock<Network>>, inv: Inv) -> Result<()> {
    println!(
        "handle_inv: Receive inventory with {} of  {}",
        inv.items.len(),
        inv.inv_type
    );

    let first_item = inv.items[0].clone();
    if inv.inv_type == *"block" {
        {
            network.write().unwrap().blocks_in_transit = inv.items;
        }

        send_get_data(
            network.clone(),
            &inv.addr_from,
            "block".to_owned(),
            first_item.clone(),
        )
        .await?;

        let mut new_in_transit = vec![];
        let blocks_in_transit = { network.read().unwrap().blocks_in_transit.clone() };

        for block_hash in blocks_in_transit {
            if block_hash == first_item {
                new_in_transit.push(block_hash);
            }
        }
    }

    if inv.inv_type == *"tx" && {
        network
            .read()
            .unwrap()
            .memory_pool
            .get(&hex::encode(first_item.clone()))
            .is_none()
    } {
        send_get_data(network, &inv.addr_from, "tx".to_owned(), first_item).await?;
    }

    Ok(())
}

async fn handle_transaction(
    network: Arc<RwLock<Network>>,
    transaction: Transaction,
    chain: Arc<RwLock<blockchain::BlockChain>>,
) -> Result<()> {
    println!("handle_transaction");

    let tx: blockchain::Transaction = bincode::deserialize(&transaction.tx)?;
    let tx_id = tx.id.clone();

    {
        network
            .write()
            .unwrap()
            .memory_pool
            .insert(hex::encode(tx_id.clone()), tx);
    }

    let node_address = { network.read().unwrap().node_address.clone() };
    let known_nodes = { network.read().unwrap().known_nodes.clone() };
    let memory_pool_size = { network.read().unwrap().memory_pool.len() };
    let mine_address = { network.read().unwrap().mine_address.clone() };

    println!("Network {} - pool size {}", node_address, memory_pool_size);

    if node_address == CENTRAL {
        for node in known_nodes {
            if node != node_address && node != transaction.addr_from {
                send_inv(network.clone(), &node, "tx".to_owned(), vec![tx_id.clone()]).await?;
            }
        }
    } else if memory_pool_size >= 1 && !mine_address.is_empty() {
        mine_tx(network, chain).await?
    }

    Ok(())
}

async fn mine_tx(
    network: Arc<RwLock<Network>>,
    chain: Arc<RwLock<blockchain::BlockChain>>,
) -> Result<()> {
    let mut txs = vec![];

    let node_address = { network.read().unwrap().node_address.clone() };
    let known_nodes = { network.read().unwrap().known_nodes.clone() };
    let memory_pool = { network.read().unwrap().memory_pool.clone() };
    let mine_address = { network.read().unwrap().mine_address.clone() };

    for (id, tx) in memory_pool.iter() {
        println!("Tx: {:?}", id);
        let verified_tx = { chain.read().unwrap().verify_transaction(tx) };
        if verified_tx.is_ok() {
            txs.push(tx.clone());
        }
    }

    if txs.is_empty() {
        println!("All transactions are invalid");
        return Ok(());
    }

    let coinbase_tx = blockchain::Transaction::coinbase_tx(&mine_address)?;
    txs.push(coinbase_tx);

    let block = { chain.write().unwrap().mine_block(txs.clone())? };
    let chain_clone = { chain.read().unwrap().clone() };
    let utxo_set = blockchain::UTXOSet::new(chain_clone);
    utxo_set.reindex()?;

    println!("New block mined");

    for tx in txs {
        let tx_id = hex::encode(tx.id);
        {
            network.write().unwrap().memory_pool.remove(&tx_id);
        }
    }

    for node in known_nodes {
        if node != node_address {
            send_inv(
                network.clone(),
                &node,
                "block".to_owned(),
                vec![block.hash.clone()],
            )
            .await?;
        }
    }

    Ok(())
}

async fn handle_version(
    network: Arc<RwLock<Network>>,
    version: Version,
    chain: Arc<RwLock<blockchain::BlockChain>>,
) -> Result<()> {
    println!("handle_version");

    let best_height = { chain.read().unwrap().get_best_height()? };
    let other_height = version.best_height;

    match best_height.cmp(&other_height) {
        Ordering::Less => send_get_blocks(network.clone(), &version.addr_from).await?,
        Ordering::Greater => send_version(network.clone(), &version.addr_from, chain).await?,
        _ => {}
    }

    let known_nodes = { network.read().unwrap().known_nodes.clone() };
    if !known_nodes.contains(&version.addr_from) {
        {
            network.write().unwrap().known_nodes.push(version.addr_from);
        }
    }

    Ok(())
}

async fn handle_connection(
    network: Arc<RwLock<Network>>,
    mut socket: TcpStream,
    chain: Arc<RwLock<blockchain::BlockChain>>,
) -> Result<()> {
    let mut buffer = [0; 8192];
    let bytes_read = socket.read(&mut buffer).await?;

    let payload = &buffer[..bytes_read];
    let command: Command = bincode::deserialize(payload)?;

    match command {
        Command::Block(block) => handle_block(network, block, chain).await?,
        Command::Inv(inv) => handle_inv(network, inv).await?,
        Command::GetBlocks(get_blocks) => handle_get_blocks(network, get_blocks, chain).await?,
        Command::GetData(get_data) => handle_get_data(network, get_data, chain).await?,
        Command::Transaction(transaction) => {
            handle_transaction(network, transaction, chain).await?
        }
        Command::Version(version) => handle_version(network, version, chain).await?,
    }

    Ok(())
}

pub async fn start_server(node_id: &str, miner_address: &str) -> Result<()> {
    let node_address = format!("127.0.0.1:{}", node_id);

    let network = Arc::new(RwLock::new(Network {
        node_address: node_address.clone(),
        mine_address: miner_address.to_owned(),
        known_nodes: vec![CENTRAL.to_owned()],
        blocks_in_transit: vec![],
        memory_pool: HashMap::new(),
    }));

    let chain = Arc::new(RwLock::new(blockchain::BlockChain::continue_blockchain(
        node_id,
    )?));

    let listener = TcpListener::bind(node_address.clone()).await?;

    if node_address != CENTRAL {
        send_version(network.clone(), CENTRAL, chain.clone()).await?;
    }

    loop {
        let (socket, _) = listener.accept().await.unwrap();
        let network_clone = network.clone();
        let chain_clone = chain.clone();

        tokio::spawn(handle_connection(network_clone, socket, chain_clone));
    }
}

pub fn send_transaction_central(node_id: &str, tx: &blockchain::Transaction) -> Result<()> {
    println!("send_transaction_central");
    let node_address = format!("127.0.0.1:{}", node_id);

    let transaction = Transaction {
        addr_from: node_address,
        tx: bincode::serialize(tx)?,
    };
    let request = bincode::serialize(&Command::Transaction(transaction))?;
    let mut socket = std::net::TcpStream::connect(CENTRAL)?;
    socket.write_all(&request)?;
    Ok(())
}
