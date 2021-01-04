use serde::{Deserialize, Serialize};

use crate::parse::input_data::InputData;

#[derive(Serialize, Deserialize, Debug)]
pub struct Transaction {
    transaction: web3::types::Transaction,
    input_data: InputData,
}

impl Transaction {
    pub fn new(transaction: web3::types::Transaction, input_data: InputData) -> Self {
        Transaction {
            transaction,
            input_data,
        }
    }
}