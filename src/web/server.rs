use std::error::Error;
use std::sync::Arc;

use actix_web::{App, HttpServer, middleware, Responder, web};
use actix_web::web::resource;
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
        Ok(res) => (),
        Err(e) => {
            error!("Failed to save contract. {:?}", e);
            return "Failed to save contract".to_string();
        }
    };

    tokio::spawn(async move {
        let fun = async { cp.get_mongo().find_trx_to(&contract.address).await };
        // todo: panic, error?
        cp.process_contract(&contract, fun).await.expect("Success");
    });

    format!("ABI saved successfully. Address: {}", address)
}
