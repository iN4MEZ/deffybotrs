use std::time::Duration;

use deffy_bot_macro::event;
use deffy_bot_utils::builder_utils::ModalBuilder;
use serenity::all::{Context, CreateInteractionResponse, CreateInteractionResponseMessage};

use crate::{command::system::manager::COOLDOWN_MANAGER, event::manager::EventData};

#[event(e = interaction_create)]
async fn on_message(ctx: Context, data: EventData) {
    if let EventData::Interaction(interaction) = data {
        if let Some(btn) = &interaction.as_message_component() {
            match btn.data.custom_id.as_str() {
                "btn1" => {
                    let cd_state = COOLDOWN_MANAGER.lock().await;

                    match cd_state
                        .check_and_update(btn.user.id.into(), Duration::from_secs(30))
                        .await
                    {
                        Ok(_) => {
                            let modal = ModalBuilder::new("verify_patreon", "Verify your email")
                                .add_text_input(
                                    "email",
                                    "Email",
                                    serenity::all::InputTextStyle::Paragraph,
                                )
                                .build();

                            let response = btn.create_response(&ctx.http, modal).await;

                            if let Err(e) = response {
                                tracing::error!("{}", e)
                            }
                        }
                        Err(e) => {
                            let _response = btn
                                .create_response(
                                    &ctx.http,
                                    CreateInteractionResponse::Message(
                                        CreateInteractionResponseMessage::new()
                                            .content(format!("too fast! {:?}",e)).ephemeral(true),
                                    ),
                                )
                                .await;
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
