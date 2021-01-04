use anyhow::{bail, Error, Result};
use elasticsearch::{BulkParts, Elasticsearch, IndexParts};
use elasticsearch::http::request::JsonBody;
use elasticsearch::http::transport::Transport;
use log::{debug, info};
use serde_json::Value;
use web3::types::Transaction;

use crate::mongo::model::Contract;
use crate::mongo::MongoDB;
use crate::parse::contract_abi::ContractAbi;
use crate::parse::trx;

mod transaction;


pub struct ContractProcessor<'a> {
    mongo: &'a MongoDB,
    elastic: &'a Elastic,
}

impl<'a> ContractProcessor<'a> {
    pub fn new(mongo: &'a MongoDB, elastic: &'a Elastic) -> Self {
        ContractProcessor {
            mongo,
            elastic,
        }
    }

    pub async fn process_contract(&self, contract: &Contract) -> Result<()> {
        let results = self.mongo.save_contract(contract).await?;
        // todo: find all trx where 'to' is our Contract.address.
        // todo: improve to Batching
        let transactions: Vec<Transaction> = self.mongo.find_trx_to(&contract.address).await?;
        // todo: parse trx. Add details from Contract ABI
        let map = trx::create_id_method_map(&contract.abi_json);

        let mut data = vec![];
        for trx in transactions {
            let input = trx::parse_trx(&map, &contract.abi_json, trx.input.to_hex::<String>().as_ref());
            let trx = transaction::Transaction::new(trx, input);
            data.push(trx);
        }

        let res = self.elastic.save_trx(data).await?;

        info!("Data saved to ES: {}", res);

        Ok(())
    }
}

pub struct Elastic {
    es: Elasticsearch
}

impl Elastic {
    pub fn new(url: &str) -> Self {
        let es = create_connection(url).expect("Can't connect to ES");

        Elastic {
            es
        }
    }

    pub async fn save_trx(&self, transactions: Vec<transaction::Transaction>) -> Result<bool> {
        let mut body: Vec<JsonBody<_>> = Vec::with_capacity(transaction.len());

        info!("Saving to ES {} trx", transactions.len());

        for trx in transactions {
            body.push(JsonBody::from(trx));
        }

        let response = self.es.bulk(BulkParts::Index("transactions"))
            .body(body)
            .send()
            .await?;

        let response_body = response.json::<Value>().await?;
        let successful = response_body["errors"].as_bool().unwrap() == false;

        Ok(successful)
    }

    pub fn create_index(&self) -> Result<()> {
        // todo: create index
        Ok(())
    }

    pub fn save(&self, index: &str) -> Result<()> {
        // todo: save to index
        Ok(())
    }
}

pub fn create_connection(url: &str) -> Result<Elasticsearch> {
    info!("Connection to ES. [{}]", url);

    let transport = Transport::single_node(url)?;

    Ok(Elasticsearch::new(transport))
}

#[cfg(test)]
mod tests {
    #[test]
    fn create_connection() {
        let con = super::create_connection("http://localhost:9200")
            .expect("Can't connect to ES");

        println!("INFO:\n{:?}", con.info());
    }
}