use std::ops::Range;

use mongodb::bson;
use mongodb::bson::Document;
use serde::{Deserialize, Serialize};
use web3::types::{Bytes, H160, H2048, H256, H64, Index, U256, U64};

use crate::parse::contract_abi::ContractAbi;
use crate::traversal::ChainData;
use std::fmt::{Display, Formatter};

#[derive(Debug, Serialize, Deserialize)]
pub struct Contract {
    #[serde(rename = "_id")]
    pub id: String,
    pub address: String,
    pub abi_json: ContractAbi,
}

impl From<Document> for Contract {
    fn from(doc: Document) -> Self {
        bson::from_document(doc).unwrap()
    }
}

impl Contract {
    pub const COLLECTION_NAME: &'static str = "contracts";

    pub fn new(address: &str, abi_json: ContractAbi) -> Self {
        Contract {
            id: address.into(),
            address: address.into(),
            abi_json,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Block {
    #[serde(rename = "_id")]
    pub id: i64,
    pub hash: Option<H256>,
    /// Hash of the parent
    #[serde(rename = "parentHash")]
    pub parent_hash: H256,
    /// Hash of the uncles
    #[serde(rename = "sha3Uncles")]
    pub uncles_hash: H256,
    /// Miner/author's address.
    #[serde(rename = "miner")]
    pub author: H160,
    /// State root hash
    #[serde(rename = "stateRoot")]
    pub state_root: H256,
    /// Transactions root hash
    #[serde(rename = "transactionsRoot")]
    pub transactions_root: H256,
    /// Transactions receipts root hash
    #[serde(rename = "receiptsRoot")]
    pub receipts_root: H256,
    /// Block number. None if pending.
    pub number: Option<U64>,
    /// Gas Used
    #[serde(rename = "gasUsed")]
    pub gas_used: U256,
    /// Gas Limit
    #[serde(rename = "gasLimit")]
    pub gas_limit: U256,
    /// Extra data
    #[serde(rename = "extraData")]
    pub extra_data: Bytes,
    /// Logs bloom
    #[serde(rename = "logsBloom")]
    pub logs_bloom: Option<H2048>,
    /// Timestamp
    pub timestamp: U256,
    /// Difficulty
    pub difficulty: U256,
    /// Total difficulty
    #[serde(rename = "totalDifficulty")]
    pub total_difficulty: Option<U256>,
    /// Seal fields
    #[serde(default, rename = "sealFields")]
    pub seal_fields: Vec<Bytes>,
    /// Uncles' hashes
    pub uncles: Vec<H256>,
    /// Transactions
    pub transactions: Vec<web3::types::Transaction>,
    /// Transactions count
    pub transactions_count: i32,
    /// Size in bytes
    pub size: Option<U256>,
    /// Mix Hash
    #[serde(rename = "mixHash")]
    pub mix_hash: Option<H256>,
    /// Nonce
    pub nonce: Option<H64>,
}

impl From<Document> for Block {
    fn from(doc: Document) -> Self {
        bson::from_document(doc).unwrap()
    }
}

impl From<&web3::types::Block<web3::types::Transaction>> for Block {
    fn from(block: &web3::types::Block<web3::types::Transaction>) -> Self {
        Block::new(block)
    }
}

pub fn extract_transactions(block: &mut Block) -> Vec<Transaction> {
    let timestamp = block.timestamp;

    // let transactions = block.transactions;

    let transactions: Vec<Transaction> = block.transactions.iter()
        .map(|t| Transaction::new(t, timestamp))
        .collect();

    block.transactions = vec![];

    transactions
}


/*pub fn create_block_trx_do(block: &web3::types::Block<web3::types::Transaction>) -> (Block, Vec<Transaction>) {
    let mut block = Block::new(block);

    let timestamp = block.timestamp;

    let transactions = block.transactions;
    block.transactions = vec![];

    let transactions: Vec<Transaction> = transactions.into_iter()
        .map(|t| Transaction::new(t, timestamp))
        .collect();

    (block, transactions)
}
*/
impl Block {
    pub const COLLECTION_NAME: &'static str = "blocks";

    pub fn new(block: &web3::types::Block<web3::types::Transaction>) -> Self {
        let transactions_count = block.transactions.len() as i32;
        Block {
            id: block.number.expect("Created block expected").as_u64() as i64,
            hash: block.hash,
            parent_hash: block.parent_hash,
            uncles_hash: block.uncles_hash,
            author: block.author,
            state_root: block.state_root,
            transactions_root: block.transactions_root,
            receipts_root: block.receipts_root,
            number: block.number,
            gas_used: block.gas_used,
            gas_limit: block.gas_limit,
            extra_data: block.extra_data.to_owned(),
            logs_bloom: block.logs_bloom,
            timestamp: block.timestamp,
            difficulty: block.difficulty,
            total_difficulty: block.total_difficulty,
            seal_fields: block.seal_fields.to_owned(),
            uncles: block.uncles.to_owned(),
            transactions: block.transactions.to_owned(),
            transactions_count,
            size: block.size,
            mix_hash: block.mix_hash,
            nonce: block.nonce,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Transaction {
    pub timestamp: U256,
    pub hash: H256,
    /// Nonce
    pub nonce: U256,
    /// Block hash. None when pending.
    #[serde(rename = "blockHash")]
    pub block_hash: Option<H256>,
    /// Block number. None when pending.
    #[serde(rename = "blockNumber")]
    pub block_number: U64,
    /// Transaction Index. None when pending.
    #[serde(rename = "transactionIndex")]
    pub transaction_index: Option<Index>,
    /// Sender
    pub from: H160,
    /// Recipient (None when contract creation)
    pub to: Option<H160>,
    /// Transfered value
    pub value: U256,
    /// Gas Price
    #[serde(rename = "gasPrice")]
    pub gas_price: U256,
    /// Gas amount
    pub gas: U256,
    /// Input data
    pub input: Bytes,
    /// Raw transaction data
    #[serde(default)]
    pub raw: Option<Bytes>,
}

impl Transaction {
    pub const COLLECTION_NAME: &'static str = "transactions";
}

impl From<Document> for Transaction {
    fn from(doc: Document) -> Self {
        bson::from_document(doc).unwrap()
    }
}

impl Transaction {
    pub fn new(trx: &web3::types::Transaction, timestamp: U256) -> Self {
        Transaction {
            timestamp,
            hash: trx.hash,
            nonce: trx.nonce,
            block_hash: trx.block_hash,
            block_number: trx.block_number.expect("Existing block"),
            transaction_index: trx.transaction_index,
            from: trx.from,
            to: trx.to,
            value: trx.value,
            gas_price: trx.gas_price,
            gas: trx.gas,
            input: trx.input.to_owned(),
            raw: trx.raw.to_owned(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChainDataDO {
    range: Range<u64>,
    pub blocks: Vec<Block>,
    pub transactions: Vec<Transaction>,
}

impl From<&ChainData> for ChainDataDO {
    fn from(cd: &ChainData) -> Self {
        let mut blocks: Vec<Block> = cd.blocks.iter()
            .map(|b| b.get_block().into())
            .collect();

        let transactions = blocks.iter_mut()
            .flat_map(|b| extract_transactions(b))
            .collect();

        ChainDataDO {
            range: cd.range.clone(),
            blocks,
            transactions,
        }
    }
}

impl Display for ChainDataDO {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,
               "Chain data: Range: {:?}, Blocks: {}", self.range, self.blocks.len())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test() {
        println!("Hello");
    }
}
