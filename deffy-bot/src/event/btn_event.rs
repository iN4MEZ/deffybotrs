use std::{any::Any, sync::{Arc, Mutex}};

use deffy_bot_macro::event;
use serenity::{all::{Context, CreateInteractionResponse, CreateInteractionResponseMessage}};

use crate::event::start_event::COMMAND_MANAGER;

#[event(e = interaction_create)]
async fn on_message(ctx: Context, data: Arc<Mutex<Box<dyn Any + Send + Sync>>>) {
    let interaction = data.lock().unwrap();
    if let Some(interaction_ref) = interaction.downcast_ref::<serenity::model::prelude::Interaction>() {
        let interaction = interaction_ref.clone();
        let ctx_clone = ctx.clone();
        tokio::spawn(async move {
            if let Some(btn) = &interaction.as_message_component() {
                match btn.data.custom_id.as_str() {
                    "btn1" => {
                        tracing::debug!("btn1 clicked");

                        let handler_opt = {
                            let guard = COMMAND_MANAGER.lock().unwrap();
                            guard.get_handler("test")
                        };

                        if let Some(cmd) = handler_opt {
                            tracing::debug!("{:?}",cmd.name());

                            let cmd_ctx = ctx_clone.clone();

                            let cmd_interaction = interaction.as_message_component().cloned();

                            if let Some(cmd_inter) = cmd_interaction {
                                if let Err(e) = cmd.execute_component(cmd_ctx, cmd_inter).await {
                                    tracing::error!("{e}");
                                }
                            }
                        }

                        let response = btn.create_response
                        (ctx_clone.http, CreateInteractionResponse::Message
                            (CreateInteractionResponseMessage::new()
                            .content("btn1 clicked").ephemeral(true)))
                            .await;

                        if let Err(e) = response {
                            tracing::error!("{}",e)
                        }

                    }
                    _ => {

                    }
                    
                }
                
            }
        });
    }

}