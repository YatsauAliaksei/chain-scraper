use web3::types::H160;

pub(crate) mod trx;
pub(crate) mod contract_abi;
pub(crate) mod input_data;

pub fn h160_to_address(address: Option<&H160>) -> String {
    format!("{:#x}", address.unwrap())
}
