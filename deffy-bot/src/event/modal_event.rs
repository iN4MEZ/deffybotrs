
use deffy_bot_macro::event;
use deffy_bot_utils::{ModalBuilder, PatreonVerification};
use serenity::all::{Context, CreateInteractionResponse, CreateInteractionResponseMessage};

use crate::event::manager::EventData;

#[event(e = interaction_create)]
async fn on_message(ctx: Context, data: EventData) {
    if let EventData::Interaction(interaction) = data {
        if let Some(modal) = &interaction.modal_submit() {
            match modal.data.custom_id.as_str() {
                "verify_patreon" => {
                    // Collect all input text values from the modal
                    let input_values = ModalBuilder::extract_modal_inputs(modal);

                    tracing::debug!("{:?}", input_values);

                    let patreon_email = input_values.iter().find(|(key, _)| key == "email").map(|(_, value)| value.clone()).unwrap_or_default();

                    let patreon_verification = PatreonVerification::new(patreon_email.clone());

                    let is_verified = patreon_verification.verify().await.unwrap_or(false);

                    if is_verified {
                        
                    }

                    let content = format!(
                        "Verified: {}", is_verified
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