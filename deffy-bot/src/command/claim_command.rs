use std::env;

use deffy_bot_macro::command;
use deffy_bot_patreon_services::PatreonApi;
use serenity::{
    all::{
        CommandInteraction, Context, CreateCommand, CreateInteractionResponse,
        CreateInteractionResponseMessage,
    },
    async_trait, Error,
};

use crate::command::manager::CommandHandler;

#[command(cmd = claim)]
pub struct ClaimCommand;

#[async_trait]
impl CommandHandler for ClaimCommand {
    async fn execute(
        &self,
        ctx: Context,
        interaction: CommandInteraction,
    ) -> Result<(), Error> {
        let content = format!("Key: ZX",);

            // Api Client 
            let api = PatreonApi {
                access_token: env::var("PATREON_ACCESS_TOKEN")
                    .expect("PATREON_ACCESS_TOKEN must be set"),
                ..Default::default()
            };

            let result = api.identity_include_memberships().await;

            if let Ok(data) = result {
                tracing::info!("{:?}", data.1);
            }

            let response = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(content)
                    .ephemeral(true),
            );

            interaction.create_response(ctx.http, response).await
            
    }

    fn register(&self) -> CreateCommand {
        CreateCommand::new("claim").description("Claim a your key")
    }
}