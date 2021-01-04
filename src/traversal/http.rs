use std::any::Any;
use std::fmt::{Display, Formatter};
use std::future::Future;
use std::ops::{Not, Range, Sub};
use std::sync::{Arc, Barrier, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{bail, Result};
use futures;
use futures::future::TryFutureExt;
use hex::encode;
use log::{debug, info};
use rayon::prelude::*;
use thiserror::Error;
use web3::types::{Address, Block, Transaction, U64};
use web3::Web3;

pub type Transport = web3::transports::Either<web3::transports::WebSocket, web3::transports::Http>;

#[derive(Debug, Clone)]
pub struct BlockExtended {
    block: Block<Transaction>,
}

impl BlockExtended {
    pub fn get_block(self) -> Block<Transaction> {
        self.block
    }
}

impl From<Block<Transaction>> for BlockExtended {
    fn from(block: Block<Transaction>) -> Self {
        BlockExtended { block }
    }
}

#[derive(Debug)]
pub struct ChainData {
    range: Range<u64>,
    blocks: Vec<BlockExtended>,
}

impl ChainData {
    pub fn new(range: Range<u64>, blocks: Vec<BlockExtended>) -> Self {
        ChainData {
            range,
            blocks,
        }
    }

    pub fn get_blocks(&self) -> &Vec<BlockExtended> {
        &self.blocks
    }
}

impl Display for ChainData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,
               "Chain data: Range: {:?}, Blocks: {}", self.range, self.blocks.len())
    }
}

#[derive(Error, Debug)]
enum ScrapingError {
    #[error("Wrong input. Range: {0:?}")]
    WrongInputError(Range<u64>),
}

pub async fn traversal(url: &str, range: Range<u64>) -> Result<ChainData> {
    if range.start > range.end {
        bail!(ScrapingError::WrongInputError(range));
    }

    let web3 = create_web3(url).await;

    info!("Starting range: {:?}", range);

    #[cfg(feature = "debug_count")]
        let mut counter = 0u32;

    let mut blocks: Vec<BlockExtended> = vec![];

    for current_block in range.clone() {
        let eth_val: U64 = current_block.into();
        let block_id = eth_val.into();

        let block = web3.eth().block_with_txs(block_id).await?.expect("block expected");

        /*        for trx in &block.transactions {
                    let from: Address = trx.from;
                    let to = trx.to;
                    let input_hex = hex::encode(&trx.input.0);

                    info!("Found trx: {:?}", trx);
                    info!("Address from: {}", from);
                    info!("Address to: {:?}", to);
                    info!("Block num: {:?}", block.number.expect("existing block"));
                    info!("Input: {}\n", input_hex);
                }
        */

        if !block.transactions.is_empty() {
            debug!("Found block [{}] with {} trx", block.number.unwrap(), block.transactions.len());
            blocks.push(block.into());
        }

        #[cfg(feature = "debug_count")]
            {
                counter += 1;
                if counter % 1000 == 0 {
                    info!("Block number: {}", current_block);
                }
            }
    }

    Ok(ChainData {
        range,
        blocks,
    })
}

pub async fn traversal_parallel(url: &str, range: Range<u64>, batch_size: u64) -> Result<ChainData> {
    if range.start > range.end {
        bail!(ScrapingError::WrongInputError(range));
    }

    let web3 = Arc::new(create_web3(url).await);

    let thread_pool = rayon::ThreadPoolBuilder::new().num_threads(num_cpus::get()).build().unwrap();

    let ranges = super::batch::create_ranges(&range, batch_size);

    let (tx_job, rx_job) = std::sync::mpsc::channel();
    let blocks = Arc::new(Mutex::new(vec![]));
    let mut jobs_counter = ranges.len();

    for range in ranges {
        let web3_cloned = web3.clone();
        let tx_job = tx_job.clone();
        let blocks = blocks.clone();

        thread_pool.spawn(move || {
            info!("Starting range: {:?}", range);

            for block_id in range.clone() {
                let b: U64 = block_id.into();

                let block = futures::executor::block_on(web3_cloned.eth().block_with_txs(b.into())).expect("Block expected");

                if block.is_none() {
                    break;
                }

                let block = block.unwrap();

                if !block.transactions.is_empty() {
                    debug!("Found block [{}] with {} trx", block.number.unwrap(), block.transactions.len());
                    blocks.lock().unwrap().push(block.into());
                }
            }

            tx_job.send(range).unwrap();
        });
    }


    for range in rx_job {
        info!("Job finished msg received. {:?}", range);
        jobs_counter -= 1;

        if jobs_counter == 0 {
            break;
        }
    }

    let mut guard = blocks.lock().unwrap();
    let length = guard.len();
    let mut blocks = vec![];

    blocks.extend(guard.drain(0..length));

    Ok(ChainData {
        range,
        blocks,
    })
}

pub async fn create_web3(url: &str) -> web3::Web3<Transport> {
    let transport = create_transport(url).await;
    web3::Web3::new(transport)
}

async fn create_transport(url: &str) -> Transport {
    match url {
        u if u.starts_with("http") => {
            info!("Creating http connection for [{}]", url);
            web3::transports::Either::Right(web3::transports::Http::new(url).unwrap())
        }
        u if u.starts_with("ws") => {
            info!("Creating ws connection for [{}]", url);
            web3::transports::Either::Left(web3::transports::WebSocket::new(url).await.unwrap())
        }
        _ => panic!("Unsupported transport")
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use env_logger::{Builder, Env};

    use super::*;

    #[tokio::test]
    async fn traversal() -> Result<()> {
        // Builder::from_env(Env::default().default_filter_or("info")).init();
        log4rs::init_file("config/log4rs.yml", Default::default()).unwrap();

        let now = std::time::Instant::now();

        let res = match super::traversal("ws://localhost:8546", 55_000..58_000).await {
            Ok(res) => res,
            Err(e) => {
                println!("Error: {}", e);
                panic!("Failed to connect")
            }
        };

        println!("Total time: {:?}", (std::time::Instant::now() - now).as_secs());
        println!("Result:\n{}", res);

        Ok(())
    }

    #[tokio::test]
    async fn traversal_parallel() -> Result<()> {
        // Builder::from_env(Env::default().default_filter_or("info")).init();
        log4rs::init_file("config/log4rs.yml", Default::default()).unwrap();

        let now = std::time::Instant::now();

        let res = match super::traversal_parallel("ws://localhost:8546", 55_000..58_004, 100).await {
            Ok(res) => res,
            Err(e) => {
                println!("Error: {}", e);
                panic!("Failed to connect")
            }
        };

        println!("Total time: {:?}", (std::time::Instant::now() - now).as_secs());
        println!("{}", res);

        Ok(())
    }
}
