use std::sync::Arc;

use actix_web::{App, HttpServer, middleware, Responder, web};
use actix_web::web::resource;
use futures::stream::StreamExt;
use log::{error, info};

use crate::es::ContractProcessor;
use crate::mongo::model::Contract;
use crate::parse::contract_abi::ContractAbi;

pub async fn run_server(cp: Arc<ContractProcessor>, port: u16) -> tokio::io::Result<()> {
    info!("Starting server on port: {}", port);

    let factory = move || {
        App::new()
            .data(cp.clone())
            .wrap(middleware::Logger::default())
            .service(resource("/abi/upload/{address}").route(web::post().to(abi_upload)))
    };

    HttpServer::new(factory).bind(format!("0.0.0.0:{}", port))?.run().await
}

// todo: handle parsing errors
async fn abi_upload(address: web::Path<String>, contract_abi: web::Json<ContractAbi>, cp: web::Data<Arc<ContractProcessor>>) -> impl Responder {
    info!("Received /abi/upload/{}", address);

    let contract = Contract::new(address.as_str(), contract_abi.into_inner());

    info!("Parsed contract: {:?}", contract);

    let contract = Arc::new(contract);
    let contract_cloned = contract.clone();
    let cp_cloned = cp.clone();

    match futures::executor::block_on(async {
        cp_cloned.save_contract(&contract_cloned).await
    }) {
        Ok(_res) => (),
        Err(e) => {
            error!("Failed to save contract. {:?}", e);
            return "Failed to save contract".to_string();
        }
    };

    tokio::spawn(async move {
        let batch_size = 10_000usize;
        info!("Starting saving trx to es for contract: {}", contract.address);

        let mut cursor = cp.get_mongo().find_trx_to(&contract.address, batch_size as u32).await.expect("Cursor expected");

        let mut bucket = Vec::with_capacity(batch_size as usize);
        let mut total = 0;

        while let Some(data) = cursor.next().await {
            bucket.push(data.unwrap().into());

            if bucket.len() == batch_size {
                cp.process_contract(&contract, bucket.iter()).await.unwrap();
                total += batch_size;
                bucket.clear();
            }
        }

        if bucket.len() > 0 {
            cp.process_contract(&contract, bucket.iter()).await.unwrap();
            total += bucket.len();
        }

        info!("ES data saved. Size: {}", total);
    });

    format!("ABI saved successfully. Address: {}", address)
}
