use crate::{blockchain::BlockChain, wallet::Wallets, Error, Result};
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
    pub fn run(&mut self) -> Result<()> {
        match Command::from_str(self.args.first().unwrap()).unwrap() {
            Command::CreateBlockchain => {
                if self.args.len() < 2 {
                    print_usage_and_exit()
                }
                self.create_blockchain(&self.args[1])?;
            }
            Command::SendCoin => {
                if self.args.len() < 4 {
                    print_usage_and_exit()
                }
                let amount = match self.args[3].parse::<u64>() {
                    Ok(amount) => amount,
                    Err(_) => {
                        return Err(Error::CustomError("send amount must be integer".to_owned()))
                    }
                };
                self.send_coin(&self.args[1], &self.args[2], amount)?;
            }
            Command::GetBalance => {
                if self.args.len() < 2 {
                    print_usage_and_exit()
                }
                self.get_balance(&self.args[1])?;
            }
            Command::PrintBlockchain => self.print_blockchain()?,
            Command::CreateWallet => self.create_wallet()?,
            Command::ListAddresses => self.list_addresses()?,
        }
        Ok(())
    }

    fn create_blockchain(&self, address: &str) -> Result<()> {
        let _ = BlockChain::init_blockchain(address.to_owned())?;
        println!("Blockchain created!");

        Ok(())
    }

    fn send_coin(&self, from: &str, to: &str, amount: u64) -> Result<()> {
        if amount == 0 {
            return Err(Error::CustomError(
                "Amount must be bigger than 0".to_owned(),
            ));
        }

        let mut chain = BlockChain::continue_blockchain()?;

        let tx = chain.new_txs(from, to, amount)?;
        chain.add_block(vec![tx])?;

        println!("Send from {} -> {} success!", from, to);

        Ok(())
    }

    fn get_balance(&self, address: &str) -> Result<()> {
        let chain = BlockChain::continue_blockchain()?;

        let mut balance = 0u64;
        let utxos = chain.find_utxo(address)?;

        for output in utxos {
            balance += output.value()
        }

        println!("Balance of {}: {}", address, balance);

        Ok(())
    }

    fn print_blockchain(&self) -> Result<()> {
        let chain = BlockChain::continue_blockchain()?;
        let mut iter = chain.iterator();

        println!("Print blockchain");
        while iter.next_print()?.is_some() {}

        Ok(())
    }

    fn create_wallet(&self) -> Result<()> {
        let mut wallets = Wallets::create_wallets()?;
        let address = wallets.add_wallet();
        wallets.save_file()?;

        println!("New wallet address {:?}", address);

        Ok(())
    }

    fn list_addresses(&self) -> Result<()> {
        let wallets = Wallets::create_wallets()?;
        let addresses = wallets.get_addresses();

        println!("Addresses");
        for address in addresses {
            println!("{}", address);
        }
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
            command => Err(format!("Invalid command {}", command).to_owned()),
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
        }
    }
}

fn print_usage_and_exit() {
    println!("Usage:");
    //? blockchain
    println!(
        " {} ADDRESS (str) - init blockchain and send genesis reward to ADDRESS",
        Command::CreateBlockchain
    );
    println!(
        " {} FROM (str) - TO (str) - AMOUNT (int) - send amount of coins",
        Command::SendCoin
    );
    println!(
        " {} ADDRESS - get balance for the ADDRESS",
        Command::GetBalance
    );
    println!(
        " {} - show all the blocks in the blockchain",
        Command::PrintBlockchain
    );

    //? wallet
    println!(" {} - create a new wallet", Command::CreateWallet);
    println!(" {} - list all the addresses", Command::ListAddresses);
    process::exit(0);
}
