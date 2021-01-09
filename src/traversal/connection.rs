use log::info;

pub type Transport = web3::transports::Either<web3::transports::WebSocket, web3::transports::Http>;


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