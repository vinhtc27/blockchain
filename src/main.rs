use blockchain::{blockchain::BlockChain, Result};
use std::env;
use std::process;

struct CommandLine {
    blockchain: BlockChain,
}

impl CommandLine {
    fn print_usage(&self) {
        println!("Usage:");
        println!(" add -block BLOCK_DATA - add a block to the chain");
        println!(" print - Prints the blocks in the chain");
    }

    fn validate_args(&self) {
        if env::args().len() < 2 {
            self.print_usage();
            process::exit(1);
        }
    }

    fn add_block(&mut self, data: &str) -> Result<()> {
        println!("Add Block!");
        self.blockchain.add_block(data)
    }

    fn print_chain(&self) -> Result<()> {
        let mut iter = self.blockchain.iterator();

        while let Some(()) = iter.next()? {}

        Ok(())
    }

    fn run(&mut self) -> Result<()> {
        self.validate_args();

        let args: Vec<String> = env::args().collect();

        match args[1].as_str() {
            "add" => {
                if args.len() < 3 {
                    self.print_usage();
                    process::exit(1);
                }
                self.add_block(&args[2])?;
            }
            "print" => self.print_chain()?,
            _ => {
                self.print_usage();
                process::exit(1);
            }
        }
        Ok(())
    }
}

fn main() -> Result<()> {
    let mut args = env::args();
    let _ = args.next(); // Skip program name

    let blockchain = BlockChain::init_blockchain()?;
    let mut cli = CommandLine { blockchain };

    cli.run()?;

    Ok(())
}
