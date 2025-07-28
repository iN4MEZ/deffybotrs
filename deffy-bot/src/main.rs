use std::{env, net::{SocketAddr}, time::Instant};
use axum::{body::Body, extract::ConnectInfo, http::Request, middleware::Next, response::Response, routing::get, Router};
use dotenv::dotenv;

mod event;
mod command;

use serenity::{all::GatewayIntents, Client};
use tokio::net::TcpListener;

use crate::event::manager::MasterHandler;

#[tokio::main(flavor = "current_thread")]
async fn main() {

    if let Err(_) = dotenv() {
        tracing::error!("Failed to load .env file");
    }
    
   tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    
    tokio::spawn(async {
        if let Err(e) = start_http().await {
            tracing::error!("Failed to start HTTP server: {:?}", e);
        }
    });


    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment").to_string();
    
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(&token, intents).event_handler(MasterHandler).await.expect("Error creating client");

    if let Err(why) = client.start().await {
        tracing::error!("Client error: {:?}", why);
    }

}

async fn start_http() -> Result<(), std::io::Error> {
    let app = Router::new()
        .route("/", get(root));

        let addr = SocketAddr::from(([0, 0, 0, 0], 10000));

        tracing::info!("Listening on {}", addr);
        
        let listener  = TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await
}

async fn root() -> &'static str {
    "Hello, World!"
}

async fn _log_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let start = Instant::now();

    let response = next.run(req).await;
    let duration = start.elapsed();

    tracing::info!(
        "Connection: {}  Request: {} {} - Response: {} - Duration: {:?}",
        addr,
        method,
        uri,
        response.status(),
        duration
    );

    response
}
