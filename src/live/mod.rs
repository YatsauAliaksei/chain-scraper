use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use clokwerk::{Interval, ScheduleHandle, Scheduler};
use futures_util::pin_mut;
use futures_util::stream::StreamExt;
use log::{info, debug};

use crate::es::ContractProcessor;
use crate::mongo::model::{ChainDataDO, Contract};
use crate::mongo::model::Transaction;
use crate::mongo::MongoDB;
use std::ops::Range;

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

    debug!("Found contracts: {:?}", contracts);

    let max_default = 100_000_000_000 as u64;
    let max_low = contracts.iter()
        .map(|c| c.processed_range.as_ref())
        .filter(Option::is_some)
        .map(|processed_range| processed_range.as_ref().unwrap().start)
        .max().unwrap_or(-1);

    let min_high = contracts.iter()
        .map(|c| c.processed_range.as_ref())
        .filter(Option::is_some)
        .map(|processed_range| processed_range.as_ref().unwrap().end)
        .min().unwrap_or(-1);

    let mut range = match (max_low, min_high) {
        (low, _) if low != 0 => 0 as u64..low as u64,
        (0, high) => high as u64..max_default,
        (-1, -1) => 0..max_default,
        (x, y) => panic!("Not expected range {}..{}", x, y),
    };// as u64..max_low as u64;

    let to_addresses: Vec<_> = contracts.iter()
        .map(|c| c.address.clone())
        .collect();

    let total_time = Instant::now();

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

            for contract in contracts.iter_mut() {
                if let Some(trx_to_save) = address_trx.remove(&contract.address) {
                    info!("Found {} trx for {}", trx_to_save.len(), contract.address);
                    contract_processor.process_contract(contract, trx_to_save.iter()).await.expect("Success");

                    update_contract(mongodb.clone(), range.clone(), contract).await;
                }
            }
        }
    } else {
        info!("Travers in progress");
        return Ok(());
    }

    // for contract in contracts.iter_mut() {
    //     update_contract(mongodb, range, contract).await;
    // }

    info!("Total spent time: {:?}", Instant::now() - total_time);

    *crate::traversal::batch::TRAVERSE_IN_PROGRESS.lock().unwrap() = false;

    Ok(())
}

async fn update_contract(mongodb: Arc<MongoDB>, range: Range<u64>, contract: &mut Contract) {
    let start = if contract.processed_range.is_some() {
        contract.processed_range.as_ref().unwrap().start
    } else {
        range.start as i64
    };

    contract.processed_range = Some(start..range.end as i64);

    debug!("Updating contract {} with range: {:?}", contract.id, contract.processed_range);

    mongodb.update_contract(contract).await.expect("Success updating contract range");
}