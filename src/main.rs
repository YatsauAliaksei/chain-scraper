#![cfg_attr(debug_assertions, allow(dead_code, unused_imports, unused_variables))]

#[macro_use]
extern crate diesel;

use std::{env, thread};
use std::sync::Arc;

use anyhow::Result;
use diesel::prelude::*;
use env_logger::{Builder, Env};
use futures::Future;
use log::info;
use structopt::StructOpt;

use crate::es::{ContractProcessor, Elastic};

mod traversal;
mod parse;
mod web;
mod db;
mod es;
mod mongo;
mod error;

#[actix_web::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    // Builder::from_env(Env::default().default_filter_or("info")).init();

    log4rs::init_file("config/log4rs.yml", Default::default()).unwrap();

    let args = Args::from_args();

    info!("Starting chain scraper with args:\n{:?}", args);

    let mongodb = mongo::MongoDB::new(&args.mongo_url);
    mongodb.init().await?;

    let chain_data = traversal::batch::traversal(&args.chain_url, args.start_block..args.end_block, args.batch_size).await;

    if chain_data.is_some() {
        mongodb.save_chain_data(&chain_data.unwrap()).await?;
    }

    let elastic = Elastic::new(&args.es_url);

    let contract_processor =  ContractProcessor::new(mongodb, elastic);

    web::server::run_server(contract_processor, 8084).await?;

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

    #[structopt(short, long, default_value = "0")]
    start_block: u64,

    #[structopt(short, long, default_value = "100000")]
    end_block: u64,

    #[structopt(short, long, default_value = "100")]
    batch_size: u64,
}
