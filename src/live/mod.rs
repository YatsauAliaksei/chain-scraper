use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use clokwerk::{Interval, ScheduleHandle, Scheduler};
use futures_util::pin_mut;
use futures_util::stream::StreamExt;
use log::info;

use crate::es::ContractProcessor;
use crate::mongo::model::{ChainDataDO, Contract};
use crate::mongo::model::Transaction;

#[derive(Debug)]
pub struct ScheduledScraper {
    timeout_sec: u64,
    chain_url: String,
    contract_processor: Arc<ContractProcessor>,
}

impl ScheduledScraper {
    pub fn new(timeout_sec: u64, chain_url: &str, contract_processor: Arc<ContractProcessor>) -> Self {
        Self {
            timeout_sec,
            chain_url: chain_url.into(),
            contract_processor,
        }
    }

    pub async fn run(&self) -> Result<ScheduleHandle> {
        let timeout_sec = self.timeout_sec.clone() as u32;
        let contract_processor = self.contract_processor.clone();
        let url = Arc::new(self.chain_url.clone());

        find(url.clone(), contract_processor.clone()).await?;

        let handler = tokio::spawn(async move {
            let mut scheduler = Scheduler::new();

            scheduler.every(Interval::Seconds(timeout_sec)).run(move || {
                info!("Starting fetch...");

                let _ = tokio::runtime::Runtime::new().unwrap().block_on(
                    async {
                        find(url.clone(), contract_processor.clone()).await
                    }
                );
            });
            scheduler.watch_thread(Duration::from_secs(5))
        }).await;

        Ok(handler?)
    }
}

async fn find(url: Arc<String>, contract_processor: Arc<ContractProcessor>) -> Result<()> {
    let mongodb = contract_processor.get_mongo();

    let last_block = mongodb.get_last_block().await;

    let web3 = Arc::new(crate::traversal::connection::create_web3(&url).await);

    let last_block = match last_block {
        Some(block) => block.number.expect("Last block number").as_u64() + 1,
        _ => 0,
    };

    info!("Last block number: {:?}", last_block);

    let total_time = Instant::now();

    let stream = crate::traversal::batch::traversal(web3, last_block..100_000_000_000, 100).await;

    let contracts: Vec<Contract> = mongodb.get_contracts().await?;
    info!("Found contracts: {}", contracts.len());

    if stream.is_some() {
        let stream = stream.unwrap();

        pin_mut!(stream);

        while let Some(chain_data) = stream.next().await {
            let chain_data = ChainDataDO::from(&chain_data);

            mongodb.save_chain_data(&chain_data).await.expect("Wasn't able to save data to Mongo");

            let mut address_trx: HashMap<String, Vec<Transaction>> = HashMap::new();
            for trx in chain_data.transactions {
                if trx.to.is_none() {
                    continue;
                }

                let to = format!("{:#x}", trx.to.unwrap());
                let vec = address_trx.get_mut(&to);
                if vec.is_some() {
                    vec.unwrap().push(trx);
                } else {
                    let mut vec = vec![];
                    vec.push(trx);
                    address_trx.insert(to, vec);
                }
            }

            for contract in &contracts {
                if let Some(trx_to_save) = address_trx.remove(&contract.address) {
                    contract_processor.process_contract(contract, trx_to_save.iter()).await.expect("Success");
                }
            }
        }

    } else {
        info!("No blocks found since {} block", last_block);
    }

    info!("Total spent time: {:?}", Instant::now() - total_time);

    *crate::traversal::batch::TRAVERSE_IN_PROGRESS.lock().unwrap() = false;

    Ok(())
}