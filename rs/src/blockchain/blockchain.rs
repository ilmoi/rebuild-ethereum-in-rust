use crate::blockchain::block::Block;
use crate::transaction::tx_queue::TransactionQueue;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blockchain {
    pub chain: Vec<Block>, // state
}

impl Blockchain {
    pub fn new() -> Self {
        Self {
            chain: vec![Block::genesis()],
        }
    }
    pub fn add_block(&mut self, block: Block, tx_queue: &mut TransactionQueue) {
        let last_block = &self.chain[self.chain.len() - 1];
        if Block::validate_block(last_block, &block) {
            println!(
                "block {} is valid, adding to chain...",
                block.block_headers.truncated_block_headers.number
            );
            //clear processed tx from the queue
            tx_queue.clear_block_tx(&block.tx_series);

            self.chain.push(block);
        }
    }
    pub fn replace_chain(&mut self, chain: Vec<Block>) -> Result<(), String> {
        for (i, block) in chain.iter().enumerate() {
            if i != 0 {
                let last_block = &chain[i - 1];
                let is_valid = Block::validate_block(&last_block, block);
                if !is_valid {
                    return Err("failed to replace chain due to validation error.".to_owned());
                }
            }
            println!(
                "Successfully validated block {}",
                block.block_headers.truncated_block_headers.number
            );
        }
        self.chain = chain;
        println!("Successfully replaced local chain.");
        Ok(())
    }
}
