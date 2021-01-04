use std::ops::Range;
use std::sync::Arc;

use anyhow::{bail, Result};
use futures::Future;
use futures::ready;
use futures::task::{Context, Poll};
use log::{debug, info, trace, warn};
use tokio::macros::support::Pin;
use web3::types::{Block, Transaction, U64};
use web3::Web3;

use crate::traversal::http::{BlockExtended, ChainData, Transport};

async fn join_parallel<T: Send + 'static>(futures: impl IntoIterator<Item=impl Future<Output=Vec<T>> + Send + 'static>) -> Vec<T> {
    let tasks: Vec<_> = futures.into_iter().map(tokio::spawn).collect();
    // unwrap the Result because it is introduced by tokio::spawn()
    // and isn't something our caller can handle

    futures::future::join_all(tasks)
        .await
        .into_iter()
        .map(Result::unwrap)
        .flat_map(|x| { x.into_iter() })
        .collect()
}

pub fn create_ranges(range: &Range<u64>, batch_size: u64) -> Vec<Range<u64>> {
    let mut start_pos = range.start;
    let mut batches = vec![];

    while start_pos < range.end {
        let end_pos = match start_pos + batch_size {
            x if x < range.end => x,
            _ => range.end
        };

        batches.push(start_pos..end_pos);
        start_pos += batch_size;
    }

    batches
}

pub async fn traversal(url: &str, mut range: Range<u64>, batch_size: u64) -> Option<ChainData> {
    let web3 = Arc::new(crate::traversal::http::create_web3(url).await);

    let web3_cloned = web3.clone();

    let last_block = tokio::task::spawn_blocking(move || {
        futures::executor::block_on(web3_cloned.eth().block_number()).expect("result")
    }).await.expect("not null").as_u64();

    if range.start > last_block {
        return None;
    }

    if range.end > last_block {
        range.end = last_block;
        info!("Range changed to align last block in chain. {:?}", range);
    }

    // let web3 = web3.clone();

    Some(traversal_parallel(web3, range, batch_size).await)
}

async fn traversal_parallel(web3: Arc<Web3<Transport>>, range: Range<u64>, batch_size: u64) -> ChainData {
    let ranges = create_ranges(&range, batch_size);

    let jobs: Vec<_> = ranges.into_iter().map(move |range| {
        process_range(range, web3.clone())
    }).collect();

    let blocks = join_parallel(jobs.into_iter()).await;

    ChainData::new(range, blocks)
}

async fn process_range(range: Range<u64>, web3: Arc<Web3<Transport>>) -> Vec<BlockExtended> {
    let mut blocks = vec![];

    debug!("Starting range: {:?}", range);

    for block_id in range.clone() {
        let b: U64 = block_id.into();

        let web3_clone = web3.clone();

        let block = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(web3_clone.eth().block_with_txs(b.into())).expect("Block expected")
        }).await;

        let block = block.expect("Block expected");

        if block.is_none() {
            warn!("Block is None {}", block_id);

            break;
        }

        let block = block.unwrap();

        if !block.transactions.is_empty() {
            trace!("Found block [{}] with {} trx", block.number.unwrap(), block.transactions.len());
            blocks.push(block.into());
        }
    }

    debug!("Finished range: {:?} found {} blocks", range, blocks.len());

    blocks
}

#[cfg(test)]
mod tests {
    use mongodb::results::InsertManyResult;

    use crate::mongo::MongoDB;
    use crate::traversal::http::ChainData;

    use super::*;

    #[test]
    fn create_batches() {
        let ranges = create_ranges(&(0..3303), 100);
        println!("ranges: {:?}", ranges);

        assert_eq!(ranges.len(), 34);
    }

    #[tokio::test]
    async fn traversal_parallel() -> Result<()> {
        crate::error::setup_panic_handler();

        log4rs::init_file("config/log4rs.yml", Default::default()).unwrap();

        let range = 170000..173000;
        let batch_size = 100;

        let start_time = std::time::Instant::now();

        // todo: think on streaming instead of bulk op
        let chain_data = super::traversal("ws://localhost:8546", range, batch_size).await;

        println!("Total time: {:?}", (std::time::Instant::now() - start_time).as_secs());

        assert!(chain_data.is_some());
        let chain_data = chain_data.unwrap();

        println!("{}", chain_data);

        println!("{:?}", chain_data.get_blocks());

        let mongo_db = MongoDB::new("localhost");

        mongo_db.save_chain_data(&chain_data).await?;

        Ok(())
    }
}