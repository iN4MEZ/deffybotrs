use std::time::Duration;

use anyhow::Ok;
use deffy_bot_localization::tr;
use deffy_bot_macro::event;
use deffy_bot_utils::builder_utils::ModalBuilder;
use serenity::all::{
    Colour, ComponentInteraction, Context, CreateEmbed, CreateInteractionResponse,
    CreateInteractionResponseMessage, InputTextStyle,
};

use crate::{command::system::manager::COOLDOWN_MANAGER, event::manager::EventData};

#[event(e = interaction_create)]
async fn on_message(ctx: Context, data: EventData) -> Result<(), anyhow::Error> {
    if let EventData::Interaction(interaction) = data {
        if let Some(btn) = interaction.as_message_component() {
            if btn.data.custom_id == "btn:verify:patreon" {

                if let Err(_) = response_cooldown(&btn, &ctx).await {
                    return Ok(());
                }

                let modal = ModalBuilder::new("verify_patreon", "Verify your email")
                    .add_text_input("email", "Email", InputTextStyle::Paragraph)
                    .build();

                btn.create_response(&ctx.http, modal).await?;
            }

            if btn.data.custom_id == "btn:tutorial:verify" {

                if let Err(_) = response_cooldown(&btn, &ctx).await {
                    return Ok(());
                }

                let header = tr!(&btn.locale, "verify_msg_header");

                let embed = CreateEmbed::default()
                    .title(header)
                    .description(format!(
                        "{}\n\n{}\n\n{}\n\n{}\n\n{}",
                        tr!(&btn.locale, "verify_msg_00"),
                        tr!(&btn.locale, "verify_msg_01"),
                        tr!(&btn.locale, "verify_msg_02"),
                        tr!(&btn.locale, "verify_msg_03"),
                        tr!(&btn.locale, "verify_msg_04")
                    ))
                    .color(Colour::new(0xf5b400));

                let response = CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .embed(embed)
                        .ephemeral(true),
                );
                btn.create_response(&ctx.http, response).await?;
            }
        }
    }

    Ok(())
}

async fn response_cooldown(btn: &ComponentInteraction, ctx: &Context) -> Result<(), anyhow::Error> {
    let cd_state = COOLDOWN_MANAGER.lock().await;

    let cooldown = cd_state
        .check_and_update(btn.user.id.into(), Duration::from_secs(30))
        .await;

    if let Err(e) = cooldown {
        let content = format!(
            "```{} {:?}```",
            tr!(&btn.locale, "button_cooldown_error"),
            e
        );
        let response = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content(content)
                .ephemeral(true),
        );
        btn.create_response(&ctx.http, response).await?;

        return Err(anyhow::anyhow!(""));
    }

    Ok(())
}
