use std::slice::Iter;
use std::sync::Arc;

use anyhow::Result;
use elasticsearch::{BulkParts, Elasticsearch};
use elasticsearch::http::request::JsonBody;
use elasticsearch::http::transport::Transport;
use log::{debug, info, warn};
use mongodb::results::InsertOneResult;
use rustc_hex::ToHex;
use serde_json::{json, Value};

use crate::mongo::model::{Contract, Transaction};
use crate::mongo::MongoDB;
use crate::parse::trx;

mod model;


#[derive(Debug)]
pub struct ContractProcessor {
    mongo: Arc<MongoDB>,
    elastic: Arc<Elastic>,
}

impl ContractProcessor {
    pub fn new(mongo: Arc<MongoDB>, elastic: Arc<Elastic>) -> Self {
        Self {
            mongo,
            elastic,
        }
    }

    pub fn get_mongo(&self) -> Arc<MongoDB> {
        self.mongo.clone()
    }

    pub async fn save_contract(&self, contract: &Contract) -> Result<InsertOneResult> {
        self.mongo.save_contract(contract).await
    }

    pub async fn process_contract(&self, contract: &Contract, transactions: impl Into<Iter<'_, Transaction>>) -> Result<()> {
        let transactions = transactions.into();
        info!("Processing {} trx for contract {}", transactions.len(), contract.address);
        let map = trx::create_id_method_map(&contract.abi_json);

        let data = transactions
            .map(|t| {
                let input = trx::parse_trx(&map, t.input.0.to_hex::<String>().as_ref());
                model::Transaction::new(&t, input)
            })
            .collect();

        let res = self.elastic.save_trx(data).await?;

        info!("Data saved to ES: {}", res);
        Ok(())
    }
}

#[derive(Debug)]
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

    pub async fn save_trx(&self, transactions: Vec<model::Transaction>) -> Result<bool> {
        if transactions.is_empty() {
            return Ok(true);
        }

        let mut body: Vec<JsonBody<_>> = Vec::with_capacity(transactions.len());

        info!("Saving to ES {} trx", transactions.len());

        for trx in transactions {
            let res = serde_json::to_value(&trx)?;
            debug!("Putting to map: {}", res);
            body.push(json!({
            "index": {
                "_id": trx.hash
            }
            }).into());
            body.push(res.into());
        }

        let response = self.es.bulk(BulkParts::Index("transactions"))
            .body(body)
            .send()
            .await?;

        let response_body = response.json::<Value>().await?;
        let successful = response_body["errors"].as_bool().unwrap() == false;

        if !successful {
            warn!("Errors while saving to ES: {:?}", response_body)
        }

        Ok(successful)
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