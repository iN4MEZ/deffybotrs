use anyhow::{Error, Ok};
use deffy_bot_encryption::EncrytionHelper;
use deffy_bot_macro::command;
use serenity::{
    all::{
        CommandInteraction, Context, CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage
    },
    async_trait,
};

use crate::command::system::manager::{CommandHandler, CommandInfo};

#[command(cmd = key, cooldown = 0)]
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

        interaction.create_response(ctx.http, response).await?;

        Ok(())

    }

    fn register(&self) -> CreateCommand {
        CreateCommand::new(self.name()).description("A key command for testing")
    }
}
