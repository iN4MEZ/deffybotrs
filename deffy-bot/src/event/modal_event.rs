use std::{any::Any, sync::{Arc, Mutex}};

use deffy_bot_macro::event;
use deffy_bot_utils::ModalBuilder;
use serenity::all::{Context, CreateInteractionResponse, CreateInteractionResponseMessage};

#[event(e = interaction_create)]
async fn on_message(ctx: Context, data: Arc<Mutex<Box<dyn Any + Send + Sync>>>) {
    let interaction = data.lock().unwrap();
    if let Some(interaction_ref) = interaction.downcast_ref::<serenity::model::prelude::Interaction>() {
        let interaction = interaction_ref.clone();
        let ctx_clone = ctx.clone();
        tokio::spawn(async move {
            if let Some(modal) = &interaction.modal_submit() {
                match modal.data.custom_id.as_str() {
                    "myModal" => {
                        // Collect all input text values from the modal
                        let input_values = ModalBuilder::extract_modal_inputs(modal);
    
                        tracing::debug!("{:?}", input_values);
                        let content = format!(
                            "You Have Enter Something"
                        );
            
                         let response = CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().content(content).ephemeral(true));
    
                        if let Err(e) = modal.create_response(ctx_clone.http, response).await {
                            tracing::error!("{}",e)
                        }
                    }
                    _ => {
                        // Handle other custom_ids if needed
                    }
                }
            }
        });
    }
}