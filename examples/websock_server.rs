use std::env;
use wallet_service_framework::network::websock_server::WsServer;

static LOCAL_SERVER: &'static str = "127.0.0.1:9000";

#[tokio::main]
async fn main() {
    use env_logger::Env;
    env_logger::from_env(Env::default().default_filter_or("warn")).init();
    
    let bind_transport = env::args()
        .nth(1)
        .unwrap_or_else(|| LOCAL_SERVER.to_string());

    let mut ws_server = match WsServer::bind(bind_transport).await {
        Ok(server) => server,
        Err(err) => panic!("{}", err),
    };

    ws_server.listen_loop().await;
}
