use deffy_bot_localization::tr;
use deffy_bot_macro::event;
use deffy_bot_utils::{
    builder_utils::ModalBuilder,
    database::{DiscordServerDatabaseManager, PatreonVerification},
};
use serenity::all::{
    ChannelId, Colour, Context, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateMessage,
    ModalInteraction, RoleId, Timestamp,
};

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

                    let patreon_verification = PatreonVerification::new(patreon_email.clone());
                    let is_verified = patreon_verification
                        .verify()
                        .await;

                    if let Err(e) = &is_verified {
                        if e.to_string().contains("409") {
                            send_modal_response("verify_already_active_member_error", &ctx, &modal)
                                .await?;
                        }
                    }

                    if let Ok(verify) = &is_verified {
                        // Add roles

                        let email = verify.1;

                        if verify.0 {
                            let role_id = DiscordServerDatabaseManager::get_verify_roles().await;

                            if let Some(role_id) = role_id {
                                let role_id = RoleId::new(role_id);

                                if let Ok(has_role) = modal
                                    .user
                                    .has_role(&ctx.http, modal.guild_id.unwrap(), role_id)
                                    .await
                                {
                                    if !has_role {
                                        modal
                                            .member
                                            .as_ref()
                                            .unwrap()
                                            .add_role(&ctx.http, role_id)
                                            .await?;
                                        let embed = CreateEmbed::default()
                                            .title("✅ User Verified")
                                            .description(format!(
                                                "*User {} has been verified*\n*{}*",
                                                &modal.user.name,
                                                &email,
                                                ))
                                            .color(Colour::new(0x00ff04))
                                            .timestamp(Timestamp::now())
                                            .thumbnail(modal.user.avatar_url().unwrap())
                                            .footer(CreateEmbedFooter::new("Verify date"));

                                        if let Some(channel_id) =
                                            DiscordServerDatabaseManager::get_logging_channel()
                                                .await
                                        {
                                            let channel_id = ChannelId::new(channel_id);
                                            channel_id
                                                .send_message(
                                                    &ctx.http,
                                                    CreateMessage::new().embed(embed),
                                                )
                                                .await?;

                                            let response = CreateInteractionResponse::Acknowledge;

                                            modal.create_response(&ctx.http, response).await?;
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        send_modal_response("404_db_error", &ctx, &modal).await?;
                    }
                }
                _ => {
                    // Handle other custom_ids if needed
                }
            }
        }
    }
    Ok(())
}

async fn send_modal_response(
    msg_code: &str,
    ctx: &Context,
    modal: &ModalInteraction,
) -> Result<(), anyhow::Error> {
    let msg = tr!(&modal.locale, msg_code);

    let response = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content(format!("```{}```", msg))
            .ephemeral(true),
    );
    modal.create_response(&ctx.http, response).await?;

    Ok(())
}
