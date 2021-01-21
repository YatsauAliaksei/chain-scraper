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
use web3::signing::Key;

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

    let web3 = Arc::new(crate::traversal::connection::create_web3(&url).await);

    let mut contracts: Vec<Contract> = mongodb.get_contracts().await?;

    if contracts.is_empty() {
        info!("Nothing to proceed. Returning...");
        return Ok(());
    }

    info!("Found contracts: {:?}", contracts);

    let mut lowest_high = contracts.iter()
        .map(|c| c.processed_range.as_ref())
        .filter(Option::is_some)
        .map(|processed_range| processed_range.as_ref().unwrap().end)
        .min().unwrap_or(0);

    let to_addresses: Vec<_> = contracts.iter()
        .map(|c| c.address.clone())
        .collect();

    let total_time = Instant::now();

    let mut range = lowest_high as u64..100_000_000_000;

    info!("Starting range: {:?}", range);

    let stream = crate::traversal::batch::traversal(web3, to_addresses.clone(), &mut range, 10).await;

    if stream.is_some() {
        let stream = stream.unwrap();

        pin_mut!(stream);

        while let Some(chain_data) = stream.next().await {
            let mut chain_data = ChainDataDO::from(&chain_data);

            {
                let trx: Vec<_> = chain_data.transactions.into_iter()
                    .filter(|t| t.to.is_some())
                    .filter(|t| to_addresses.contains(&crate::parse::h160_to_address(t.to.as_ref())))
                    .collect();

                chain_data.transactions = trx;
            }

            mongodb.save_chain_data(&chain_data).await.expect("Wasn't able to save data to Mongo");

            let mut address_trx: HashMap<String, Vec<Transaction>> = HashMap::new();

            if contracts.is_empty() {
                // info!("Empty contract");
                continue;
            }

            for trx in chain_data.transactions {
                if trx.to.is_none() {
                    // info!("to=None");
                    continue;
                }

                let to = format!("{:#x}", trx.to.unwrap()).to_lowercase();
                let vec = address_trx.get_mut(&to);
                if vec.is_some() {
                    vec.unwrap().push(trx);
                } else {
                    let mut vec = vec![];
                    vec.push(trx);
                    address_trx.insert(to, vec);
                }
            }

            // info!("Going to contracts");

            for contract in &contracts {
                if let Some(trx_to_save) = address_trx.remove(&contract.address) {
                    info!("Found {} trx for {}", trx_to_save.len(), contract.address);
                    contract_processor.process_contract(contract, trx_to_save.iter()).await.expect("Success");
                }
            }
        }
    } else {
        info!("Travers in progress");
        return Ok(());
    }

    for contract in contracts.iter_mut() {
        let start = if contract.processed_range.is_some() {
            contract.processed_range.as_ref().unwrap().start
        } else {
            0
        };

        contract.processed_range = Some(start..range.end as i64);

        info!("Updating contract {} with range: {:?}", contract.id, contract.processed_range);

        mongodb.update_contract(contract).await.unwrap();
    }

    info!("Total spent time: {:?}", Instant::now() - total_time);

    *crate::traversal::batch::TRAVERSE_IN_PROGRESS.lock().unwrap() = false;

    Ok(())
}