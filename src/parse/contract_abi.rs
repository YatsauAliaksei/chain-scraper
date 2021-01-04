use std::fmt::Display;

use anyhow::{bail, Error, Result};
use mongodb::{bson, bson::Document};
use serde::{Deserialize, Serialize};
use serde::export::Formatter;
use serde_json::Value;

/// Creates `ContractAbi` from given Json
pub fn create_contract_abi(contract_json: &str) -> Result<ContractAbi> {
    Ok(serde_json::from_str::<ContractAbi>(contract_json)?)
}

/// Contract JSON interface
#[derive(Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub struct ContractAbi {
    pub functions: Vec<ContractFunction>
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ContractFunction {
    #[serde(default)]
    pub inputs: Vec<InOutType>,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub outputs: Vec<InOutType>,
    pub state_mutability: StateMutability,
    pub r#type: FunctionType,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StateMutability {
    NONPAYABLE,
    PAYABLE,
    VIEW,
    PURE,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ValueType {
    // value types
    UINT8,
    UINT16,
    UINT32,
    UINT64,
    UINT128,
    UINT160,
    UINT256,

    INT8,
    INT16,
    INT32,
    INT64,
    INT128,
    INT256,

    BOOL,
    ADDRESS,
    // 20 bytes
    BYTES1,// -BYTES32

    // Dynamically-sized byte array
    BYTES,
    STRING,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FunctionType {
    FUNCTION,
    CONSTRUCTOR,
    // receive ether function
    RECEIVE,
    // 'default' function
    FALLBACK,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InOutType {
    pub name: String,
    pub r#type: ValueType,
}

pub trait HasName where Self: Display {
    fn name(&self) -> String {
        self.to_string().to_ascii_lowercase()
    }
}

impl Display for FunctionType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Display for ValueType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl HasName for ValueType {}

impl HasName for FunctionType {}

#[cfg(test)]
mod tests {
    use super::*;

    const BUY_CONTRACT: &str = r###"[{"inputs":[{"internalType":"address","name":"executorAddress","type":"address"},{"internalType":"address","name":"_buyer","type":"address"},{"internalType":"uint256","name":"_amount","type":"uint256"},{"internalType":"uint256","name":"_price","type":"uint256"}],"stateMutability":"nonpayable","type":"constructor"},{"inputs":[],"name":"buy","outputs":[],"stateMutability":"nonpayable","type":"function"},{"inputs":[],"name":"getInfo","outputs":[{"internalType":"address","name":"","type":"address"},{"internalType":"uint256","name":"","type":"uint256"},{"internalType":"uint256","name":"","type":"uint256"},{"internalType":"bool","name":"","type":"bool"}],"stateMutability":"view","type":"function"}]"###;

    #[test]
    fn create_contract() {
        let contract = match super::create_contract_abi(BUY_CONTRACT) {
            Ok(c) => c,
            Err(e) => panic!(e.to_string())
        };

        assert_eq!(contract.functions.len(), 3);

        let constructor = contract.functions.get(0).unwrap();

        assert_eq!(constructor.r#type, FunctionType::CONSTRUCTOR);
        assert!(constructor.outputs.is_empty());
        println!("{:?}", contract)
    }
}