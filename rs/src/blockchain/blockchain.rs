use crate::blockchain::block::Block;

pub struct Blockchain {
    pub chain: Vec<Block>, // state
}

impl Blockchain {
    fn new() -> Self {
        Self {
            chain: vec![Block::genesis()],
        }
    }
    fn add_block(&mut self, block: Block) {
        let last_block = &self.chain[self.chain.len() - 1];
        if Block::validate_block(last_block, &block) {
            self.chain.push(block);
        }
    }
}
