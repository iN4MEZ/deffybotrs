use axum::{
    Router, body::Body, extract::ConnectInfo, http::Request, middleware::Next, response::Response,
    routing::get,
};
use dotenv::dotenv;
use std::{env, net::SocketAddr, time::Instant};

mod command;
mod event;

use serenity::{Client, all::GatewayIntents};
use tokio::{net::TcpListener, sync::mpsc};

use crate::event::manager::{MasterHandler, spawn_event_dispatcher};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(_) = dotenv() {
        tracing::error!("Failed to load .env file");
    }

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tokio::spawn(async {
        if let Err(e) = start_http().await {
            tracing::error!("Failed to start HTTP server: {:?}", e);
        }
    });

    let (tx, rx) = mpsc::channel(100);

    spawn_event_dispatcher(rx).await;

    #[cfg(not(debug_assertions))]
    {
        let db = deffy_bot_utils::database::DatabaseManager::init_db().await;

        match db {
            Ok(db) => {
                if let Err(e) = db.collect().await {
                    tracing::error!("{:?}", e)
                }
            }
            Err(err) => {
                tracing::error!("Error connect with database {}", err)
            }
        }
    }

    let token = env::var("DISCORD_TOKEN")
        .expect("Expected a token in the environment")
        .to_string();

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(&token, intents)
        .event_handler(MasterHandler { tx })
        .await
        .expect("Error creating client");

    if let Err(why) = client.start().await {
        tracing::error!("Client error: {:?}", why);
    }
}

async fn start_http() -> Result<(), std::io::Error> {
    let app = Router::new().route("/", get(root));

    let addr = SocketAddr::from(([0, 0, 0, 0], 10000));

    tracing::info!("Listening on {}", addr);

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
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
