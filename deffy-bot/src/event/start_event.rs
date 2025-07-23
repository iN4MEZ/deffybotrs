use std::{any::Any, env, sync::{Arc, Mutex}};
use deffy_bot_macro::event;
use once_cell::sync::Lazy;
use serenity::all::{Context, GuildId};

use crate::command::manager::CommandManager;

pub static COMMAND_MANAGER: Lazy<Mutex<CommandManager>> =
    Lazy::new(|| Mutex::new(CommandManager::new()));


#[event(e = ready)]
async fn on_ready(ctx: Context, _data: Arc<Mutex<Box<dyn Any + Send + Sync>>>) -> Result<(),Error> {
    let guild_id = GuildId::new(
        env::var("GUILD_ID")
            .expect("Expected GUILD_ID in environment")
            .parse()
            .expect("GUILD_ID must be an integer"),
    );

    let commands = {
        let mut manager = COMMAND_MANAGER.lock().unwrap();
        manager.register_commands();
        manager.get_commands()
    };

    let commands = guild_id.set_commands(ctx.http, commands).await;

    match commands {
        Ok(_) => tracing::info!("Commands registered successfully"),
        Err(e) => tracing::error!("Failed to register commands: {}", e),
    }

    tracing::info!("Logged in as {}", ctx.cache.current_user().name);
}