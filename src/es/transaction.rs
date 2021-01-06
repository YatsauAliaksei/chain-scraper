use std::time::SystemTime;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use web3::types::{Bytes, H160, H256, Index, U256, U64};

use crate::parse::input_data::InputData;

#[derive(Serialize, Deserialize, Debug)]
pub struct Transaction {
    pub timestamp: DateTime<Utc>,
    /// Hash
    pub hash: H256,
    /// Nonce
    pub nonce: u64,
    /// Block hash. None when pending.
    #[serde(rename = "blockHash")]
    pub block_hash: Option<H256>,
    /// Block number. None when pending.
    #[serde(rename = "blockNumber")]
    pub block_number: u64,
    /// Transaction Index. None when pending.
    #[serde(rename = "transactionIndex")]
    pub transaction_index: Option<Index>,
    /// Sender
    pub from: H160,
    /// Recipient (None when contract creation)
    pub to: Option<H160>,
    /// Transfered value
    pub value: u64,
    /// Gas Price
    #[serde(rename = "gasPrice")]
    pub gas_price: u64,
    /// Gas amount
    pub gas: u64,
    /// Input data
    pub input: Bytes,
    /// Raw transaction data
    #[serde(default)]
    pub raw: Option<Bytes>,
    pub input_data: InputData,
}

impl Transaction {
    pub fn new(trx: web3::types::Transaction, input_data: InputData) -> Self {
        let now: DateTime<Utc> = DateTime::from(SystemTime::now());
        Transaction {
            timestamp: now,
            hash: trx.hash,
            nonce: trx.nonce.as_u64(),
            block_hash: trx.block_hash,
            block_number: trx.block_number.expect("Existing block").as_u64(),
            transaction_index: trx.transaction_index,
            from: trx.from,
            to: trx.to,
            value: trx.value.as_u64(),
            gas_price: trx.gas_price.as_u64(),
            gas: trx.gas.as_u64(),
            input: trx.input,
            raw: trx.raw,
            input_data,
        }
    }
}