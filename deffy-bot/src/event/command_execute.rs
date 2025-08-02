use deffy_bot_macro::event;
use serenity::all::Context;

use crate::{command::system::{interaction_reply::InteractionExt, manager::CommandJob}, event::{manager::EventData, start_event::COMMAND_MANAGER}};

#[event(e = interaction_create)]
pub async fn on_message(ctx: Context, data: EventData) {
    if let EventData::Interaction(interaction) = data {
        if let Some(command) = interaction.as_command() {
            // 1. ดึง handler และ tx จาก COMMAND_MANAGER
            let (handler_opt, tx_opt) = {
                if let Some(manager) = COMMAND_MANAGER.get() {
                    let guard = manager.lock().await;
                    (
                        guard.get_handler(&command.data.name),
                        Some(guard.tx.clone()),
                    )
                } else {
                    (None, None)
                }
            };

            match (handler_opt, tx_opt) {
                (Some(handler), Some(tx)) => {
                    tracing::trace!("Queueing command: {}", command.data.name);

                    let job = CommandJob {
                        ctx: ctx.clone(),
                        interaction: command.clone(),
                        handler,
                    };

                    if let Err(e) = tx.send(job).await {
                        tracing::error!("Failed to send job to queue: {:?}", e);
                    }
                }

                _ => {
                    tracing::warn!("No handler found or command system uninitialized for command: {}", command.data.name);

                    let content = format!("This command is currently unavailable.");

                    let result = command.reply(&ctx, content, true).await;

                    if let Err(e) = result {
                        tracing::error!("Failed to send reply: {:?}", e);
                    }
                   
                }
            }
        }
        
    }
}
