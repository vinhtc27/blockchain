use crate::{
    blockchain::{BlockChain, Transaction, UTXOSet},
    network,
    wallet::Wallets,
    Error, Result,
};
use std::{env, fmt, process, str::FromStr};

pub struct CommandLine {
    args: Vec<String>,
}

impl Default for CommandLine {
    fn default() -> Self {
        if env::args().len() < 2 {
            print_usage_and_exit()
        }

        let args: Vec<String> = env::args().skip(1).collect();

        Self { args }
    }
}

impl CommandLine {
    pub async fn run(&mut self) -> Result<()> {
        let node_id = match env::var("NODE_ID") {
            Ok(val) => {
                println!("Value of NODE_ID: {}", val);
                val
            }
            Err(e) => panic!("Missing environment variable: {}", e),
        };

        match Command::from_str(self.args.first().unwrap()).unwrap() {
            Command::CreateBlockchain => {
                if self.args.len() < 2 {
                    print_usage_and_exit()
                }
                self.create_blockchain(&node_id, &self.args[1])?;
            }
            Command::SendCoin => {
                if self.args.len() < 5 {
                    print_usage_and_exit()
                }
                let amount = match self.args[3].parse::<u64>() {
                    Ok(amount) => amount,
                    Err(_) => {
                        return Err(Error::CustomError("Amount must be an integer".to_owned()))
                    }
                };
                let mine_now = match self.args[4].parse::<bool>() {
                    Ok(amount) => amount,
                    Err(_) => {
                        return Err(Error::CustomError("MINE_NOW must be an boolean".to_owned()))
                    }
                };
                self.send_coin(&node_id, &self.args[1], &self.args[2], amount, mine_now)?;
            }
            Command::GetBalance => {
                if self.args.len() < 2 {
                    print_usage_and_exit()
                }
                self.get_balance(&node_id, &self.args[1])?;
            }
            Command::PrintBlockchain => self.print_blockchain(&node_id)?,
            Command::CreateWallet => self.create_wallet(&node_id)?,
            Command::ListAddresses => self.list_addresses(&node_id)?,
            Command::ReindexUTXO => self.reindex_utxo(&node_id)?,
            Command::StartNode => {
                if self.args.len() < 2 {
                    print_usage_and_exit()
                }
                self.start_node(&node_id, &self.args[1]).await?
            }
        }

        println!();
        Ok(())
    }

    fn create_blockchain(&self, node_id: &str, address: &str) -> Result<()> {
        let mut wallets = Wallets::create_wallets(node_id)?;
        wallets.get_wallet(address)?;

        let chain = BlockChain::init_blockchain(node_id, address)?;
        println!("Blockchain created");

        let utxo_set = UTXOSet::new(chain);
        utxo_set.reindex()?;

        println!();
        Ok(())
    }

    fn send_coin(
        &self,
        node_id: &str,
        from: &str,
        to: &str,
        amount: u64,
        mine_now: bool,
    ) -> Result<()> {
        let mut chain = BlockChain::continue_blockchain(node_id)?;
        let utxo_set = UTXOSet::new(chain.clone());

        let tx = Transaction::new(node_id, from, to, amount, &utxo_set)?;
        if mine_now {
            let coinbase_tx = Transaction::coinbase_tx(from)?;

            let block = chain.mine_block(vec![coinbase_tx, tx])?;
            utxo_set.update(&block)?;
        } else {
            network::send_transaction_localhost(node_id, &tx)?;
            println!("Send transaction");
        }
        println!("Send {amount} coin | {from} -> {to}");

        println!();
        Ok(())
    }

    fn get_balance(&self, node_id: &str, address: &str) -> Result<()> {
        let chain = BlockChain::continue_blockchain(node_id)?;

        let mut wallets = Wallets::create_wallets(node_id)?;
        wallets.get_wallet(address)?;

        let utxo_set = UTXOSet::new(chain);
        let balance = utxo_set.get_balance(address)?;

        println!("Balance of {address}: {balance}");

        println!();
        Ok(())
    }

    fn print_blockchain(&self, node_id: &str) -> Result<()> {
        let chain = BlockChain::continue_blockchain(node_id)?;
        let mut iter = chain.iterator();

        println!("Blockchain Info\n");
        while iter.next_print()?.is_some() {}

        println!();
        Ok(())
    }

    fn create_wallet(&self, node_id: &str) -> Result<()> {
        let mut wallets = Wallets::create_wallets(node_id)?;
        let address = wallets.add_wallet()?;
        wallets.save_file(node_id)?;

        println!("Wallet: {}", address);

        println!();
        Ok(())
    }

    fn list_addresses(&self, node_id: &str) -> Result<()> {
        let wallets = Wallets::create_wallets(node_id)?;
        let addresses = wallets.list_addresses();

        println!("List addresses");
        for address in addresses {
            println!("{address}");
        }

        println!();
        Ok(())
    }

    fn reindex_utxo(&self, node_id: &str) -> Result<()> {
        let chain = BlockChain::continue_blockchain(node_id)?;
        let utxo_set = UTXOSet::new(chain);
        utxo_set.reindex()?;

        let count = utxo_set.count_transaction();
        println!("Reindex UTXO set with {count} transaction");

        println!();
        Ok(())
    }

    async fn start_node(&self, node_id: &str, miner_address: &str) -> Result<()> {
        println!("Starting node {node_id}");

        let mut wallets = Wallets::create_wallets(node_id)?;

        if !miner_address.is_empty() {
            if wallets.get_wallet(miner_address)?.is_some() {
                println!("Mining is on. Address to receive rewards: {miner_address}");
            } else {
                println!("Wrong miner address!");
            }
        }

        network::start_server(node_id, miner_address).await?;

        println!();
        Ok(())
    }
}

enum Command {
    CreateBlockchain,
    SendCoin,
    GetBalance,
    PrintBlockchain,
    CreateWallet,
    ListAddresses,
    ReindexUTXO,
    StartNode,
}

impl FromStr for Command {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "create_blockchain" => Ok(Command::CreateBlockchain),
            "send_coin" => Ok(Command::SendCoin),
            "get_balance" => Ok(Command::GetBalance),
            "print_blockchain" => Ok(Command::PrintBlockchain),
            "create_wallet" => Ok(Command::CreateWallet),
            "list_addresses" => Ok(Command::ListAddresses),
            "reindex_utxo" => Ok(Command::ReindexUTXO),
            "start_node" => Ok(Command::StartNode),
            _ => {
                println!("Invalid command!\n");
                print_usage_and_exit();
                Err(String::new())
            }
        }
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Command::CreateBlockchain => write!(f, "create_blockchain"),
            Command::SendCoin => write!(f, "send_coin"),
            Command::GetBalance => write!(f, "get_balance"),
            Command::PrintBlockchain => write!(f, "print_blockchain"),
            Command::CreateWallet => write!(f, "create_wallet"),
            Command::ListAddresses => write!(f, "list_addresses"),
            Command::ReindexUTXO => write!(f, "reindex_utxo"),
            Command::StartNode => write!(f, "start_node"),
        }
    }
}

fn print_usage_and_exit() {
    println!("USAGE:");
    println!(
        " {} ADDRESS (str) | init blockchain and send genesis reward to ADDRESS",
        Command::CreateBlockchain
    );
    println!(
        " {} FROM (str) - TO (str) - AMOUNT (int) - MINE_NOW (bool) | send amount of coins",
        Command::SendCoin
    );
    println!(
        " {} ADDRESS | get balance for the ADDRESS",
        Command::GetBalance
    );
    println!(
        " {} | show all the blocks in the blockchain",
        Command::PrintBlockchain
    );
    println!(" {} | create a new wallet", Command::CreateWallet);
    println!(" {} | list all the addresses", Command::ListAddresses);
    println!(" {} | rebuild the UTXO set", Command::ReindexUTXO);
    println!(
        " {} - MINER (str) | start anode with id specified in NODE_ID env and enable miner option",
        Command::StartNode
    );
    process::exit(0);
}
