use deffy_bot_macro::event;
use deffy_bot_utils::{builder_utils::ModalBuilder, database::{DiscordServerDatabaseManager, PatreonVerification}};
use serenity::all::{Context, CreateInteractionResponse, CreateInteractionResponseMessage, RoleId};

use crate::event::manager::EventData;

#[event(e = interaction_create)]
async fn on_message(ctx: Context, data: EventData) -> Result<(), anyhow::Error> {
    if let EventData::Interaction(interaction) = data {
        if let Some(modal) = &interaction.modal_submit() {
            match modal.data.custom_id.as_str() {
                "verify_patreon" => {
                    // Collect all input text values from the modal
                    let input_values = ModalBuilder::extract_modal_inputs(modal);

                    tracing::debug!("{:?}", input_values);

                    let patreon_email = input_values
                        .iter()
                        .find(|(key, _)| key == "email")
                        .map(|(_, value)| value.clone())
                        .unwrap_or_default();

                    let is_verified = PatreonVerification::new(patreon_email.clone())
                        .verify()
                        .await;

                    if let Ok(verify) = is_verified {
                        // Add roles

                        if verify {
                            let role_id = DiscordServerDatabaseManager::get_verify_roles().await;

                            if let Some(role_id) = role_id {
                                let role_id = RoleId::new(role_id);

                                if let Ok(has_role) = modal
                                    .user
                                    .has_role(&ctx.http, modal.guild_id.unwrap(), role_id)
                                    .await
                                {
                                    if !has_role {
                                        tracing::debug!("Role Added!");
                                        if let Err(err) = modal
                                            .member
                                            .as_ref()
                                            .unwrap()
                                            .add_role(&ctx.http, role_id)
                                            .await
                                        {
                                            tracing::error!("{err}");
                                        }
                                    }
                                }
                            } else {
                                let content = format!("Database Error: Role not found");

                                let response = CreateInteractionResponse::Message(
                                    CreateInteractionResponseMessage::new()
                                        .content(content)
                                        .ephemeral(true),
                                );
                                modal
                                    .create_response(&ctx.http, response)
                                    .await?;
                            }
                        }
                    }

                    let content = format!("Verified: {:?}", is_verified);

                    let response = CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content(content)
                            .ephemeral(true),
                    );

                    modal.create_response(&ctx.http, response).await?;
                }
                _ => {
                    // Handle other custom_ids if needed
                }
            }
        }
    }
    Ok(())
}
