use std::collections::HashMap;
use std::fmt::Display;
use std::str::FromStr;

use anyhow::Result;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use tiny_keccak::{Hasher, Keccak};

use crate::parse::contract_abi::{ContractAbi, ContractFunction, FunctionType, HasName, InOutType};
use crate::parse::input_data::{InputData};
use serde_json::{Value, Map};

use super::contract_abi::ValueType;

const BYTE_LENGTH: usize = 32 << 1;


fn parse_address(trx_raw_input: &str, offset: usize) -> String {
    String::from(&trx_raw_input[offset + 24..offset + BYTE_LENGTH])
}

fn parse_int(trx_raw_input: &str, offset: usize) -> u64 {
    u64::from_str_radix(&trx_raw_input[offset..offset + BYTE_LENGTH], 16).expect("u64 in hex")
}

fn parse_bool(trx_raw_input: &str, offset: usize) -> bool {
    let num = parse_int(trx_raw_input, offset);
    num == 1
}

// 'bytes' will work only if can be represented as utf8 so no diff with 'string' type
fn dynamic_type(trx_raw_input: &str, offset: usize) -> Map<String, Value> {
    let location = &trx_raw_input[offset..BYTE_LENGTH + offset];
    let location = usize::from_str_radix(location, 16).expect("Location as u64 expected") << 1;

    let len = &trx_raw_input[8 + location..8 + location + BYTE_LENGTH];
    let len = usize::from_str_radix(len, 16).expect("u64 in hex") << 1;

    let data_hex = &trx_raw_input[8 + location + BYTE_LENGTH..8 + location + BYTE_LENGTH + len];
    let data = hex::decode(data_hex).expect("Hex in str");
    let data = String::from_utf8(data).expect("String expected");

    serde_json::from_str(&data).expect("Orig value failed")
}

fn get_method_id(signature: &str) -> String {
    // let mut sha = Sha3::v256();
    let mut sha = Keccak::v256();

    sha.update(signature.as_bytes());
    let mut hash = [0; 64];
    sha.finalize(&mut hash[..]);
    hex::encode(hash)
}

pub fn parse_trx(id_method: &HashMap<String, &ContractFunction>, trx_raw_input: &str) -> InputData {
    let trx_raw_input = trx_raw_input.strip_prefix("0x").unwrap_or(trx_raw_input);
    debug!("input: {:?}", trx_raw_input);

    let mut offset = 0;
    let method_id = &trx_raw_input[offset..8];
    offset += 8;

    let function = id_method.get(&method_id.to_string()).expect(&format!("method expected: {}", method_id));

    debug!("Method: {:?}", function);
    let mut args = Map::with_capacity(function.inputs.len());

    for input in &function.inputs {
        let value = match &input.r#type {
            ValueType::ADDRESS => Value::from(parse_address(trx_raw_input, offset)),
            ValueType::BOOL => Value::from(parse_bool(trx_raw_input, offset)),
            ValueType::STRING => Value::from(dynamic_type(trx_raw_input, offset)),
            ValueType::BYTES => Value::from(dynamic_type(trx_raw_input, offset)),
            int if int.to_string().starts_with("UINT") || int.to_string().starts_with("INT") =>
                Value::from(parse_int(trx_raw_input, offset)),
            _ => panic!("Unknown type: {:?}", input),
        };
        offset += BYTE_LENGTH;

        args.insert(input.name.clone(), value);
    }

    InputData::new(function.name.clone().as_str(), args)
}

fn build_method_sig(function: &ContractFunction) -> Option<String> {
    if function.r#type == FunctionType::CONSTRUCTOR {
        return None;
    }

    let mut sig = function.name.clone();
    sig.push_str("(");
    for i in &function.inputs {
        sig.push_str(&i.r#type.name());
        sig.push(',');
    }

    if sig.ends_with(",") {
        sig.remove(sig.len() - 1);
    }

    sig.push_str(")");
    Some(sig)
}

pub fn create_id_method_map(contract: &ContractAbi) -> HashMap<String, &ContractFunction> {
    let mut id_to_method = HashMap::new();

    for f in &contract.functions {
        let sig = build_method_sig(f);

        if sig.is_none() { continue; }

        let mut id = get_method_id(&sig.unwrap());

        id.truncate(8);
        id_to_method.insert(id, f);
    }

    id_to_method
}

#[cfg(test)]
mod tests {
    use std::collections::hash_map::RandomState;
    use std::collections::HashMap;
    use std::str::FromStr;

    use super::*;

    const BUY_CONTRACT: &str = r###"[{"inputs":[{"internalType":"address","name":"executorAddress","type":"address"},{"internalType":"address","name":"_buyer","type":"address"},{"internalType":"uint256","name":"_amount","type":"uint256"},{"internalType":"uint256","name":"_price","type":"uint256"}],"stateMutability":"nonpayable","type":"constructor"},{"inputs":[],"name":"buy","outputs":[],"stateMutability":"nonpayable","type":"function"},{"inputs":[],"name":"getInfo","outputs":[{"internalType":"address","name":"","type":"address"},{"internalType":"uint256","name":"","type":"uint256"},{"internalType":"uint256","name":"","type":"uint256"},{"internalType":"bool","name":"","type":"bool"}],"stateMutability":"view","type":"function"}]"###;

    const SCRAPER_TESTING_CONTRACT: &str = r#"[{"inputs":[{"internalType":"uint256","name":"_amount","type":"uint256"},{"internalType":"uint256","name":"_price","type":"uint256"}],"stateMutability":"nonpayable","type":"constructor"},{"inputs":[],"name":"getInfo","outputs":[{"internalType":"uint256","name":"","type":"uint256"},{"internalType":"uint256","name":"","type":"uint256"}],"stateMutability":"view","type":"function"},{"inputs":[{"internalType":"uint256","name":"_amount","type":"uint256"}],"name":"newAmount","outputs":[],"stateMutability":"nonpayable","type":"function"},{"inputs":[{"internalType":"uint256","name":"_price","type":"uint256"}],"name":"newPrice","outputs":[],"stateMutability":"nonpayable","type":"function"},{"inputs":[{"internalType":"string","name":"userData","type":"string"},{"internalType":"bytes","name":"clientData","type":"bytes"}],"name":"submit","outputs":[],"stateMutability":"nonpayable","type":"function"}] "#;

    #[test]
    fn parse_int() {
        let num = super::parse_int("0000000000000000000000000000000000000000000000000000000000000080", 0);
        assert_eq!(num, 128);
    }

    #[test]
    fn parse_address() {
        let address = super::parse_address("0000000000000000000000007001ea1ca8c28aa90a0d2e8b034aa56319ff0a7e", 0);
        assert_eq!(address, "7001ea1ca8c28aa90a0d2e8b034aa56319ff0a7e");
    }

    #[test]
    fn parse_bool() {
        let b = super::parse_bool("0000000000000000000000000000000000000000000000000000000000000001", 0);
        assert!(b);
        let b = super::parse_bool("0000000000000000000000000000000000000000000000000000000000000000", 0);
        assert!(!b);
    }

    #[test]
    fn build_method_sig() {
        crate::error::setup_panic_handler();

        log4rs::init_file("config/log4rs.yml", Default::default()).unwrap();

        let con = crate::parse::contract_abi::create_contract_abi(SCRAPER_TESTING_CONTRACT).unwrap();

        let submit = &con.functions[4];

        let sig = super::build_method_sig(submit).unwrap();
        assert_eq!(sig, "submit(string,bytes)");

        let id = super::get_method_id(&sig);
        let expected: [u8; 4] = [158, 129, 63, 31];

        let expected = hex::encode(expected);

        assert_eq!(id[..8], expected);
    }

    const SUBMIT_TRX_HEX: &str = r#"0x9e813f1f0000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000187b226964223a3133322c226e616d65223a22416c6578227d000000000000000000000000000000000000000000000000000000000000000000000000000000207b22746178223a3133322c226e756d626572223a22555549442d31323334227d"#;

    #[test]
    fn parse_trx() {
        crate::error::setup_panic_handler();

        log4rs::init_file("config/log4rs.yml", Default::default()).unwrap();

        let con = crate::parse::contract_abi::create_contract_abi(SCRAPER_TESTING_CONTRACT)
            .unwrap();

        let id_method = super::create_id_method_map(&con);

        info!("map: {:?}", id_method);

        let input_data = super::parse_trx(&id_method, SUBMIT_TRX_HEX);
        info!("Result: {:?}", input_data);

        assert_eq!("submit", input_data.method_name());
        assert_eq!(format!("{:?}", input_data.args().get("clientData").unwrap()), r#"{"tax":132,"number":"UUID-1234"}"#);
        assert_eq!(format!("{:?}", input_data.args().get("userData").unwrap()), r#"{"id":132,"name":"Alex"}"#);
    }
}