use deffy_bot_encryption::EncrytionHelper;
use deffy_bot_macro::command;
use serenity::{
    Error,
    all::{
        CommandInteraction, Context, CreateCommand, CreateInteractionResponse,
        CreateInteractionResponseMessage,
    },
    async_trait,
};

use crate::command::manager::{CommandHandler, CommandInfo};

#[command(cmd = key)]
pub struct KeyCommand;

#[async_trait]
impl CommandHandler for KeyCommand {
    async fn execute(&self, ctx: Context, interaction: CommandInteraction) -> Result<(), Error> {
        let enc = EncrytionHelper::encrypt("hello");

        let content = format!("{}. Key: {}", interaction.user.name, enc);

        let response = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content(content)
                .ephemeral(true),
        );

        let result = interaction.create_response(ctx.http, response).await;

        match result {
            Ok(_) => {
                tracing::info!("Interaction responded successfully");
                Ok(())
            }
            Err(e) => {
                tracing::error!("Failed to respond to interaction: {:?}", e);
                Err(e)
            }
        }
    }
    fn register(&self) -> CreateCommand {
        CreateCommand::new(self.name()).description("A key command for testing")
    }
}
