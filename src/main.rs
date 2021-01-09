#![cfg_attr(debug_assertions, allow(dead_code, unused_imports, unused_variables))]

#[macro_use]
extern crate lazy_static;

use std::sync::Arc;

use anyhow::Result;
use log::info;
use structopt::StructOpt;

use crate::es::{ContractProcessor, Elastic};
use crate::live::ScheduledScraper;
use crate::error::setup_panic_handler;

mod traversal;
mod parse;
mod web;
// mod db;
mod es;
mod mongo;
mod error;
mod live;

#[actix_web::main]
async fn main() -> Result<()> {
    setup_panic_handler();
    dotenv::dotenv().ok();
    // Builder::from_env(Env::default().default_filter_or("info")).init();

    log4rs::init_file("config/log4rs.yml", Default::default()).unwrap();

    let args = Args::from_args();

    info!("Starting chain scraper with args:\n{:?}", args);

    let elastic = Arc::new(Elastic::new(&args.es_url));

    let mongodb = mongo::MongoDB::new(&args.mongo_url);
    mongodb.init().await?;

    let mongodb = Arc::new(mongodb);

    // traversal::batch::init(&args.chain_url, mongodb.clone(), args.start_block..args.end_block, args.batch_size).await?;

    let contract_processor = Arc::new(ContractProcessor::new(mongodb.clone(), elastic.clone()));

    let scheduled_scraper = ScheduledScraper::new(60, &args.chain_url, contract_processor.clone());

    let _handler = scheduled_scraper.run().await?;


    web::server::run_server(contract_processor.clone(), args.listen_port).await?;

    Ok(())
}

#[derive(Debug, StructOpt)]
#[structopt(about = "Chain Scrapper")]
struct Args {
    #[structopt(long = "chain_url", default_value = "ws://localhost:8546")]
    chain_url: String,

    #[structopt(long = "mongo_url", default_value = "localhost")]
    mongo_url: String,

    #[structopt(long = "elastic_url", default_value = "http://localhost:9200")]
    es_url: String,

    #[structopt(short = "p", long, default_value = "8084")]
    listen_port: u16,

    #[structopt(short, long, default_value = "0")]
    start_block: u64,

    #[structopt(short, long, default_value = "100000")]
    end_block: u64,

    #[structopt(short, long, default_value = "100")]
    batch_size: u64,
}
