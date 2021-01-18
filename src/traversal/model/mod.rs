use std::fmt::{Display, Formatter};
use std::ops::Range;

use web3::types::{Block, Transaction};

#[derive(Debug)]
pub struct ChainData {
    pub range: Range<u64>,
    pub blocks: Vec<Block<Transaction>>,
}

impl ChainData {
    pub fn new(range: Range<u64>, blocks: Vec<Block<Transaction>>) -> Self {
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
