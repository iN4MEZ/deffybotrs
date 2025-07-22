use std::{
    any::Any, env, sync::{Arc, Mutex}
};

use handler_macro::event;
use once_cell::sync::Lazy;
use serenity::all::{Context, CreateInteractionResponse, CreateInteractionResponseMessage, GuildId};

pub static COMMAND_MANAGER: Lazy<Mutex<CommandManager>> =
    Lazy::new(|| Mutex::new(CommandManager::new()));

use crate::command::{
    command_registry::CommandManager
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
        manager.register_command(crate::command::test_command::TestCommand);
        manager.register_command(crate::command::key_command::KeyCommand);
        manager.register_command(crate::command::profile_command::ProfileCommand);
        manager.register_command(crate::command::claim_command::ClaimCommand);
        manager.register_command(crate::command::modal_command::ModalCommand);
        manager.register_command(crate::command::embed_command::EmbedCommand);
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

            let handler_opt = {
                let guard = COMMAND_MANAGER.lock().unwrap();
                guard.get_handler(&command.data.name)
            };

            handler_opt.map_or_else(
                || {
                    tracing::warn!("No handler found for command: {}", command.data.name);
                },
                |handler| {
                    tracing::trace!("Executing command: {}", command.data.name);
                    let ctx_clone = ctx.clone();
                    let interaction_clone = interaction.clone();
                    tokio::spawn(async move {

                        let interaction = match interaction_clone.as_command() {
                            Some(c) => c.clone(),
                            None => {
                                tracing::error!("Interaction is not a command");
                                return;
                            }
                        };

                        let interaction_hander_clone = interaction.clone();

                        if let Err(e) = handler.execute(ctx_clone, interaction_hander_clone).await {
                            tracing::error!("Error executing command: {}", e);

                            let _rsp = interaction.create_response
                            (ctx.http, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().content(format!("An Command Error: {:?}",e)).ephemeral(true))).await;
                        }
                    });
                },
            );
        }
    }
}
