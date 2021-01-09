use std::ops::Range;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use anyhow::Result;
use futures::Future;
use log::{debug, info, trace, warn};
use web3::futures::TryFutureExt;
use web3::types::U64;
use web3::Web3;

use crate::traversal::{BlockExtended, ChainData};
use crate::traversal::connection::Transport;

lazy_static! {
    pub static ref TRAVERSE_IN_PROGRESS: Mutex<bool> = Mutex::new(false);
}

async fn join_parallel<T: Send + 'static>(futures: impl IntoIterator<Item=impl Future<Output=Vec<T>> + Send + 'static>) -> Vec<T> {
    let tasks: Vec<_> = futures.into_iter().map(tokio::spawn).collect();

    futures::future::join_all(tasks)
        .await
        .into_iter()
        .map(Result::unwrap)
        .flat_map(|x| { x.into_iter() })
        .collect()
}

fn create_ranges(range: &Range<u64>, batch_size: u64) -> Vec<Range<u64>> {
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

pub async fn traversal(web3: Arc<Web3<Transport>>, mut range: Range<u64>, batch_size: u64) -> Option<ChainData> {
    if *TRAVERSE_IN_PROGRESS.lock().unwrap() {
        debug!("Travers in progress");
        return None;
    } else {
        *TRAVERSE_IN_PROGRESS.lock().unwrap() = true;
    }

    let last_block = web3.eth().block_number().into_future().await.expect("last block result").as_u64();

    if range.start > last_block {
        return None;
    }

    if range.end > last_block {
        range.end = last_block;
        info!("Range changed to align last block in chain. {:?}", range);
    }

    Some(traversal_parallel(web3, range, batch_size).await)
}

async fn traversal_parallel(web3: Arc<Web3<Transport>>, init_range: Range<u64>, batch_size: u64) -> ChainData {
    let ranges = create_ranges(&init_range, 100_000);

    info!("Range: {:?}. {} ranges started with size: {}. Sub range size: {}", init_range, ranges.len(), 100_000, batch_size);

    let total_time = Instant::now();
    let mut result = vec![];

    for range in ranges {
        let web3 = web3.clone();
        let range_start_time = Instant::now();

        let sub_ranges = create_ranges(&range, 100);
        let sub_ranges_len = sub_ranges.len();

        let jobs: Vec<_> = sub_ranges.into_iter()
            .map(move |range| {
                process_range(range, web3.clone())
            }).collect();

        let blocks = join_parallel(jobs.into_iter()).await;

        info!("Range {:?} finished. {} sub-ranges processed in {}ms. Blocks found : {}", range, sub_ranges_len, (Instant::now() - range_start_time).as_millis(), blocks.len());
        result.extend(blocks);
    }

    info!("Total spent time: {:?}", Instant::now() - total_time);

    ChainData::new(init_range, result)
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
    use crate::traversal::ChainData;

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
        let mongo_db = Arc::new(MongoDB::new("localhost"));

        // todo: think on streaming instead of bulk op
        let web3 = Arc::new(crate::traversal::connection::create_web3(url).await);
        let cd = super::traversal(web3, range, batch_size).await?;

        println!("Total time: {:?}", (std::time::Instant::now() - start_time).as_secs());

        Ok(())
    }
}