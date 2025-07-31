use std::env;

use anyhow::{Error, Ok};
use deffy_bot_macro::command;
use deffy_bot_patreon_services::PatreonApi;
use serenity::{
    all::{
        CommandInteraction, Context, CreateCommand, CreateInteractionResponse,
        CreateInteractionResponseMessage, CreateMessage,
    },
    async_trait,
};

use crate::command::system::manager::CommandHandler;

#[command(cmd = claim,cooldown = 0)]
pub struct ClaimCommand;

#[async_trait]
impl CommandHandler for ClaimCommand {
    async fn execute(&self, ctx: Context, interaction: CommandInteraction) -> Result<(), Error> {
        let content = format!("Key: ZX",);

        // Api Client
        let api = PatreonApi {
            access_token: env::var("PATREON_ACCESS_TOKEN")
                .expect("PATREON_ACCESS_TOKEN must be set"),
            ..Default::default()
        };

        let data = api.all_members().await?;

        for mem in data {
            interaction
                .channel_id
                .send_message(
                    &ctx.http,
                    CreateMessage::new().content(format!("{:?}", mem.attributes.email)),
                )
                .await?;
        }

        let response = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content(content)
                .ephemeral(true),
        );

        interaction.create_response(ctx.http, response).await?;

        Ok(())
    }

    fn register(&self) -> CreateCommand {
        CreateCommand::new("claim").description("Claim a your key")
    }
}
