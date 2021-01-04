use std::sync::Arc;

use actix_web::{App, HttpMessage, HttpRequest, HttpServer, middleware, post, Responder, web};
use actix_web::dev::Server;
use actix_web::web::resource;
use log::{debug, info};

use crate::db;
use crate::es::ContractProcessor;
use crate::mongo::model::Contract;
use crate::mongo::MongoDB;
use crate::parse::contract_abi::ContractAbi;

pub async fn run_server(cp: ContractProcessor, port: u64) -> tokio::io::Result<()> {
    let cp = Arc::new(cp);
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

    tokio::spawn(async move {
        cp.process_contract(&contract).await
    });

    format!("ABI saved successfully. Address: {}", address)
}
