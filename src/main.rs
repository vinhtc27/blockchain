use blockchain::{blockchain::BlockChain, Result};
use std::{env, process};

struct CommandLine {}

impl CommandLine {
    fn print_usage(&self) {
        println!("Usage:");
        println!(" create ADDRESS - creates a blockchain and sends genesis reward to address");
        println!(" send FROM - TO - AMOUNT - send amount of coins");
        println!(" balance ADDRESS - get balance for the address");
        println!(" print - prints the blocks in the chain");
    }

    fn validate_args(&self) {
        if env::args().len() < 2 {
            self.print_usage();
            process::exit(0);
        }
    }

    fn create(&self, address: &str) -> Result<()> {
        let _ = BlockChain::init_blockchain(address.to_owned())?;
        println!("Blockchain created!");

        Ok(())
    }

    fn send(&self, from: &str, to: &str, amount: u64) -> Result<()> {
        let mut chain = BlockChain::continue_blockchain()?;

        let tx = chain.new_txs(from, to, amount)?;
        chain.add_block(vec![tx])?;

        println!("Send from {} -> {} success!", from, to);

        Ok(())
    }

    fn balance(&self, address: &str) -> Result<()> {
        let chain = BlockChain::continue_blockchain()?;

        let mut balance = 0u64;
        let utxos = chain.find_utxo(address)?;

        for output in utxos {
            balance += output.value()
        }

        println!("Balance of {}: {}", address, balance);

        Ok(())
    }

    fn print(&self) -> Result<()> {
        let chain = BlockChain::continue_blockchain()?;
        let mut iter = chain.iterator();

        println!("Print blockchain");
        while iter.next_print()?.is_some() {}

        Ok(())
    }

    fn run(&mut self) -> Result<()> {
        self.validate_args();

        let args: Vec<String> = env::args().collect();

        match args[1].as_str() {
            "create" => {
                if args.len() < 3 {
                    self.print_usage();
                    process::exit(0);
                }
                self.create(&args[2])?;
            }
            "send" => {
                if args.len() < 5 {
                    self.print_usage();
                    process::exit(0);
                }
                self.send(&args[2], &args[3], args[4].parse::<u64>().unwrap())?;
            }
            "balance" => {
                if args.len() < 3 {
                    self.print_usage();
                    process::exit(0);
                }
                self.balance(&args[2])?;
            }
            "print" => self.print()?,
            _ => {
                self.print_usage();
                process::exit(0);
            }
        }
        Ok(())
    }
}

fn main() -> Result<()> {
    let mut args = env::args();
    let _ = args.next();
    CommandLine {}.run()?;

    Ok(())
}
