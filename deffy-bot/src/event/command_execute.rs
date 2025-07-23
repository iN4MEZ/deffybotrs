use std::{
    any::Any,
    sync::{Arc, Mutex},
};
use deffy_bot_macro::event;
use serenity::{all::{
    Context, CreateInteractionResponse, CreateInteractionResponseMessage,
}};

use crate::event::start_event::COMMAND_MANAGER;

#[event(e = interaction_create)]
async fn on_message(ctx: Context, data: Arc<Mutex<Box<dyn Any + Send + Sync>>>) {
    let interaction = data.lock().unwrap();
    if let Some(interaction_ref) =
        interaction.downcast_ref::<serenity::model::prelude::Interaction>()
    {
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

                            if let Err(e) =  interaction
                                .create_response(
                                    ctx.http,
                                    CreateInteractionResponse::Message(
                                        CreateInteractionResponseMessage::new()
                                            .content(format!("An Command Error: {:?}", e))
                                            .ephemeral(true),
                                    ),
                                )
                                .await {
                                    tracing::error!("Error sending response: {}", e);
                                }
                        }
                    });
                },
            );
        }
    }
}
