use mongodb::bson;
use mongodb::bson::Document;
use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;

use crate::parse::contract_abi::ContractAbi;

#[derive(Debug, Serialize, Deserialize)]
pub struct Contract {
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
            address: address.into(),
            abi_json,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Block {
    raw_data: String,
    number: u64,
    timestamp: u64,
    transactions: Vec<String>,
}

impl From<Document> for Block {
    fn from(doc: Document) -> Self {
        bson::from_document(doc).unwrap()
    }
}

impl Block {
    pub const COLLECTION_NAME: &'static str = "blocks";

    pub fn new(raw_data: &str, number: u64, timestamp: u64, transactions: impl Into<Vec<String>>) -> Self {
        Block {
            raw_data: raw_data.into(),
            number,
            timestamp,
            transactions: transactions.into(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, TypedBuilder)]
pub struct Transaction {
    block_number: u64,
    block_hash: String,
    #[builder(default)]
    block_timestamp: Option<u64>,

    hash: String,
    // index
    from: String,
    // index
    #[builder(default)]
    to: Option<String>,
    // eth?
    value: u64,
    gas_price: u64,
    gas: u64,
    input: Vec<u8>,
}

impl Transaction {
    pub const COLLECTION_NAME: &'static str = "transactions";
}

impl From<Document> for Transaction {
    fn from(doc: Document) -> Self {
        bson::from_document(doc).unwrap()
    }
}

impl From<web3::types::Transaction> for Transaction {
    fn from(trx: web3::types::Transaction) -> Self {
        Transaction::builder()
            .block_number(trx.block_number.expect("Expected created trx").as_u64())
            .block_hash(trx.block_hash.expect("Expected created trx").to_string())
            .hash(trx.hash.to_string())
            .from(trx.from.to_string())
            .to(match trx.to {
                Some(t) => Some(t.to_string()),
                _ => None
            })
            .value(trx.value.as_u64())
            .gas_price(trx.gas_price.as_u64())
            .gas(trx.gas.as_u64())
            .input(trx.input.0)
            .build()
    }
}

