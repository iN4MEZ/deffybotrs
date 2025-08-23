use deffy_bot_http::http_init;
use deffy_bot_utils::event::manager::EVENT_MANAGER;
use dotenv::dotenv;
use std::env;
use tracing_subscriber::EnvFilter;

mod command;
mod event;

use serenity::{Client, all::GatewayIntents};
use tokio::sync::mpsc;

use crate::event::manager::{MasterHandler, spawn_event_dispatcher};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(_) = dotenv() {
        tracing::error!("Failed to load .env file");
    }

    init_logging();

    EVENT_MANAGER.lock().await.register().await;

    if let Err(e) = http_init().await {
        tracing::error!("Failed to initialize HTTP server: {:?}", e);
    }

    match init_database().await {
        Ok(_) => tracing::info!("Database initialized successfully"),
        Err(e) => tracing::error!("Failed to initialize database: {:?}", e),
    }

    match init_discord_client().await {
        Ok(client) => client,
        Err(e) => {
            tracing::error!("Failed to create Discord client: {:?}", e);
            return;
        }
    };
}

fn init_logging() {
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| {
        if cfg!(debug_assertions) {
            "trace".to_string() // default debug build
        } else {
            "info".to_string() // default release build
        }
    });

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(filter))
        .init();
}

async fn init_database() -> Result<(), anyhow::Error> {
    let db = deffy_bot_utils::database::DatabaseManager::init_db().await;

    match db {
        Ok(db) => {
            if let Err(e) = db.start_collect().await {
                tracing::error!("{:?}", e)
            }
        }
        Err(err) => {
            tracing::error!("Error connect with database {}", err)
        }
    }

    Ok(())
}

async fn init_discord_client() -> Result<(), serenity::Error> {
    let (tx, rx) = mpsc::channel(100);

    spawn_event_dispatcher(rx).await;

    let token = env::var("DISCORD_TOKEN")
        .expect("Expected a token in the environment")
        .to_string();

    let intents = GatewayIntents::all();
    let mut client = Client::builder(&token, intents)
        .event_handler(MasterHandler { tx })
        .await
        .expect("Error creating client");

    let author = env!("CARGO_PKG_AUTHORS");

    tracing::info!("[{}] DEFFY Services", author);

    client.start().await?;

    Ok(())
}
