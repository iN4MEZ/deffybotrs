use deffy_bot_http::init;
use dotenv::dotenv;
use std::env;

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

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    if let Err(e) = init().await {
        tracing::error!("Failed to initialize HTTP server: {:?}", e);
    }

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

     tracing::info!("Starting the bot...");

    if let Err(why) = client.start().await {
        tracing::error!("Client error: {:?}", why);
    }
}