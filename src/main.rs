mod blockchain;

use blockchain::BlockChain;

fn main() {
    let mut chain = BlockChain::init_blockchain();

    chain.add_block("first block");
    chain.add_block("second block");
    chain.add_block("third block");

    chain.println();
}
