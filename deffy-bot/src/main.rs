use std::env;
use dotenv::dotenv;

mod event;
mod command;

use serenity::{all::GatewayIntents, Client};

use crate::event::event_registry::MasterHandler;

#[tokio::main]
async fn main() {

    if let Err(_) = dotenv() {
        println!("Failed to load .env file");
    }
    
   tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents).event_handler(MasterHandler).await.expect("Error creating client");

    if let Err(why) = client.start().await {
        tracing::error!("Client error: {:?}", why);
    }

}
