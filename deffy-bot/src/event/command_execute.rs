use std::{
    any::Any,
    env,
    sync::{Arc, Mutex},
};

use handler_macro::event;
use once_cell::sync::Lazy;
use serenity::all::{Context, GuildId};

pub static COMMAND_MANAGER: Lazy<Mutex<CommandManager>> =
    Lazy::new(|| Mutex::new(CommandManager::new()));

use crate::command::{
    command_registry::{ CommandManager},
    message::testcommand::TestCommand,
};

#[event(e = ready)]
async fn on_ready(ctx: Context, _data: Arc<Mutex<Box<dyn Any + Send + Sync>>>) {
    let guild_id = GuildId::new(
        env::var("GUILD_ID")
            .expect("Expected GUILD_ID in environment")
            .parse()
            .expect("GUILD_ID must be an integer"),
    );

    let commands = {
        let mut manager = COMMAND_MANAGER.lock().unwrap();
        manager.register_command(TestCommand);
        manager.get_commands()
    };

    let commands = guild_id.set_commands(ctx.http, commands).await;

    match commands {
        Ok(_) => tracing::info!("Commands registered successfully"),
        Err(e) => tracing::error!("Failed to register commands: {}", e),
    }
}

#[event(e = interaction_create)]
async fn on_message(ctx: Context, data: Arc<Mutex<Box<dyn Any + Send + Sync>>>) {
    let interaction = data.lock().unwrap();
    if let Some(interaction_ref) = interaction.downcast_ref::<serenity::model::prelude::Interaction>() {
        let interaction = interaction_ref.clone();
        if let Some(command) = &interaction.as_command() {
            // You can call the command handler here if needed

            let handler_opt = {
                let guard = COMMAND_MANAGER.lock().unwrap();
                guard.get_handler(&command.data.name)
            };

            handler_opt.map_or_else(
                || {
                    tracing::warn!("No handler found for command: {}", command.data.name);
                },
                |handler| {
                    let ctx_clone = ctx.clone();
                    let interaction_clone = interaction.clone();
                    let data = Arc::new(Mutex::new(Box::new(interaction_clone) as Box<dyn Any + Send + Sync>));
                    tokio::spawn(async move {
                        handler.run(ctx_clone,data).await;
                    });
                },
            );
        } else {
            tracing::warn!("Received interaction is not a command");
        }
    } else {
        tracing::warn!("No command Matched");
    }
}
