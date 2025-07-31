
use deffy_bot_macro::event;
use deffy_bot_utils::ModalBuilder;
use serenity::all::{Context, CreateInteractionResponse, CreateInteractionResponseMessage};

use crate::event::manager::EventData;

#[event(e = interaction_create)]
async fn on_message(ctx: Context, data: EventData) {
    if let EventData::Interaction(interaction) = data {
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

                    if let Err(e) = modal.create_response(ctx.http, response).await {
                        tracing::error!("{}",e)
                    }
                }
                _ => {
                    // Handle other custom_ids if needed
                }
            }
        }
    }
}