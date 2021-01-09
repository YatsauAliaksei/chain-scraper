use std::fmt::Debug;

use anyhow::{bail, Result};
use futures::StreamExt;
use log::{debug, info, warn};
use mongodb::{bson, bson::doc, bson::Document, Client, Database};
use mongodb::options::{ClientOptions, Credential, FindOneOptions, FindOptions, StreamAddress};
use mongodb::results::{InsertManyResult, InsertOneResult};
use serde::Serialize;

use crate::mongo::model::{ChainDataDO, Transaction};

pub(crate) mod model;

#[derive(Debug)]
pub struct MongoDB {
    database: Database
}

impl MongoDB {
    pub fn new(url: &str) -> Self {
        info!("Connecting to Mongo [{}]", url);

        let database = take_db(url);

        MongoDB {
            database
        }
    }

    pub async fn get_contracts(&self) -> Result<Vec<model::Contract>> {
        let result: Vec<model::Contract> = self.find_all(model::Contract::COLLECTION_NAME, None, FindOptions::builder()
            .batch_size(100)
            .build()).await?;

        info!("Found {} contracts", result.len());

        Ok(result)
    }

    async fn find_all<D, FO, T>(&self, collection_name: &str, filter: D, find_opts: FO) -> Result<Vec<T>>
        where
            D: Into<Option<Document>> + Debug,
            FO: Into<Option<FindOptions>>,
            T: From<Document>
    {
        let collection = self.database.collection(collection_name);

        let mut cursor = match collection.find(filter, find_opts).await {
            Ok(r) => r,
            _ => return Ok(vec![]),
        };

        let mut result = vec![];
        while let Some(doc) = cursor.next().await {
            debug!("{:?}", doc);
            result.push(doc?.into());
        }
        Ok(result)
    }

    pub async fn find_trx_to(&self, address: &str) -> Result<Vec<Transaction>> {
        let collection = self.database.collection(Transaction::COLLECTION_NAME);

        let mut cursor = match collection.find(doc! {
            "to": address
        }, FindOptions::builder()
            .batch_size(100)
            .build(),
        ).await {
            Ok(r) => r,
            _ => return Ok(vec![]),
        };

        let mut result = vec![];
        while let Some(doc) = cursor.next().await {
            debug!("{:?}", doc);
            result.push(doc?.into());
        }

        info!("Found {} trx related to address: {}", result.len(), address);

        Ok(result)
    }

    pub async fn find_item(&self, collection_name: &str,
                           filter: impl Into<Option<Document>>,
                           options: impl Into<Option<FindOneOptions>>) -> Option<Document> {
        let collection = self.database.collection(collection_name);

        match collection.find_one(filter, options).await {
            Ok(r) => r,
            _ => None
        }
    }

    pub async fn get_last_block(&self) -> Option<web3::types::Block<web3::types::Transaction>> {
        let find_options = FindOneOptions::builder()
            .sort(doc! {
                        "number": -1
                    }).build();


        let doc = self.find_item(model::Block::COLLECTION_NAME, None, find_options).await;
        if doc.is_none() {
            return None;
        }

        match bson::from_document(doc.unwrap()) {
            Ok(block) => Some(block),
            Err(e) => {
                warn!("failed to parse last block request result. {:?}", e);
                None
            }
        }
    }

    pub async fn save_chain_data(&self, chain_data: &ChainDataDO) -> Result<Vec<InsertManyResult>> {
        if chain_data.blocks.is_empty() {
            info!("Nothing to save. Blocks size 0");
            return Ok(vec![]);
        }

        info!("Saving: {}", chain_data);

        let mut res = vec![];
        res.push(self.save_blocks(&chain_data.blocks).await?);
        res.push(self.save_transactions(&chain_data.transactions).await?);
        Ok(res)
    }

    pub async fn save_contract(&self, contract: &model::Contract) -> Result<InsertOneResult> {
        let contracts = self.database.collection(model::Contract::COLLECTION_NAME);

        debug!("Saving contract {:?}", contract);

        match contracts.insert_one(bson::to_document(contract)?, None).await {
            mongodb::error::Result::Ok(r) => Ok(r),
            mongodb::error::Result::Err(e) => bail!(e)
        }
    }

    pub async fn save_transactions(&self, transactions: &Vec<model::Transaction>) -> Result<InsertManyResult> {
        self.insert_many(model::Transaction::COLLECTION_NAME, transactions.iter()).await
    }

    pub async fn save_blocks(&self, blocks: &Vec<model::Block>) -> Result<InsertManyResult> {
        self.insert_many(model::Block::COLLECTION_NAME, blocks.iter()).await
    }

    pub async fn insert_many<T: Serialize + 'static>(&self, collection_name: &str, items: impl IntoIterator<Item=&T>) -> Result<InsertManyResult> {
        let items: Vec<_> = items.into_iter()
            .map(|v| { bson::to_document(v) })
            .map(std::result::Result::unwrap)
            .collect();

        let collection = self.database.collection(collection_name);

        match collection.insert_many(items, None).await {
            mongodb::error::Result::Ok(r) => Ok(r),
            mongodb::error::Result::Err(e) => bail!(e),
        }
    }

    pub async fn init(&self) -> Result<()> {
        let existed_collections = self.database.list_collection_names(None).await?;

        let collections = vec![
            model::Contract::COLLECTION_NAME,
            model::Block::COLLECTION_NAME,
            model::Transaction::COLLECTION_NAME,
        ];

        debug!("Checking collections");

        for collection in collections {
            if !existed_collections.contains(&(collection.to_string())) {
                debug!("Creating collection: {}", collection);

                self.database.create_collection(collection, None).await?;
            };
        }

        Ok(())
    }
}

pub fn create_connection(url: &str) -> Result<Client> {
    let options = ClientOptions::builder()
        .direct_connection(true)
        .credential(Credential::builder()
            .username("admin".to_string())
            .password("secret".to_string())
            .build()
        )
        .hosts(vec![
            StreamAddress {
                hostname: url.into(),
                port: Some(27017),
            }
        ])
        .build();

    Ok(Client::with_options(options)?)
}

pub fn take_db(url: &str) -> Database {
    let client = create_connection(url).expect("failed to connect to mongo");
    client.database("chain_scraper")
}

#[cfg(test)]
mod tests {
    use std::fmt::LowerHex;

    use log::info;
    use mongodb::{bson, bson::doc, bson::Document};
    use mongodb::options::{CreateCollectionOptions, DatabaseOptions, FindOneOptions, IndexOptionDefaults};

    use crate::mongo::model::Contract;
    use crate::parse::contract_abi::create_contract_abi;

    use super::*;

    const CONTRACT: &str = r#"[{"inputs":[{"internalType":"address","name":"executorAddress","type":"address"},{"internalType":"address","name":"_buyer","type":"address"},{"internalType":"uint256","name":"_amount","type":"uint256"},{"internalType":"uint256","name":"_price","type":"uint256"}],"stateMutability":"nonpayable","type":"constructor"},{"inputs":[],"name":"buy","outputs":[],"stateMutability":"nonpayable","type":"function"},{"inputs":[],"name":"getInfo","outputs":[{"internalType":"address","name":"","type":"address"},{"internalType":"uint256","name":"","type":"uint256"},{"internalType":"uint256","name":"","type":"uint256"},{"internalType":"bool","name":"","type":"bool"}],"stateMutability":"view","type":"function"}]"#;

    #[tokio::test]
    async fn get_last_block() -> Result<()> {
        crate::error::setup_panic_handler();
        log4rs::init_file("config/log4rs.yml", Default::default()).unwrap();

        let mongo_db = MongoDB::new("localhost");

        let block = mongo_db.get_last_block().await.unwrap();

        info!("Block result: {:?}", block);

        assert!(block.hash.is_some());

        Ok(())
    }

    #[tokio::test]
    async fn find_items() -> Result<()> {
        crate::error::setup_panic_handler();
        log4rs::init_file("config/log4rs.yml", Default::default()).unwrap();

        let trx_to = "0x7001ea1ca8c28aa90a0d2e8b034aa56319ff0a7e";

        let mongo_db = MongoDB::new("localhost");
        let items = mongo_db.find_trx_to(trx_to).await?;

        info!("Result: {:?}", items);

        assert!(items.len() > 0);
        assert!(items[0].to.is_some());
        assert_eq!(format!("{:#x}", items[0].to.unwrap()), trx_to);

        Ok(())
    }
}