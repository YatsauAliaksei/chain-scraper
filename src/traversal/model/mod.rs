use std::fmt::{Display, Formatter};
use std::ops::Range;

use web3::types::{Block, Transaction};

#[derive(Debug, Clone)]
pub struct BlockExtended {
    block: Block<Transaction>,
}

impl BlockExtended {
    pub fn get_block(&self) -> &Block<Transaction> {
        &self.block
    }
}

impl From<Block<Transaction>> for BlockExtended {
    fn from(block: Block<Transaction>) -> Self {
        BlockExtended { block }
    }
}

#[derive(Debug)]
pub struct ChainData {
    pub range: Range<u64>,
    pub blocks: Vec<BlockExtended>,
}

impl ChainData {
    pub fn new(range: Range<u64>, blocks: Vec<BlockExtended>) -> Self {
        ChainData {
            range,
            blocks,
        }
    }
}

impl Display for ChainData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,
               "Chain data: Range: {:?}, Blocks: {}", self.range, self.blocks.len())
    }
}
