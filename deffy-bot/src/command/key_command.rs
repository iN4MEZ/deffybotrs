
use deffy_bot_encryption::EncrytionHelper;
use serenity::{all::{CommandInteraction, Context, CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage}, async_trait, Error};

use crate::command::command_registry::{CommandHandler, CommandInfo};

pub struct KeyCommand;

#[async_trait]
impl CommandHandler for KeyCommand {
    async fn execute(&self, ctx: Context, interaction: CommandInteraction) -> Result<(), Error> {

        let enc = EncrytionHelper::encrypt("hello");

        let content = format!(
                "{}. Key: {}",
                interaction.user.name,
                enc

            );

             let response = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().content(content).ephemeral(true),
        );

        interaction
        .create_response(ctx.http, response)
        .await
        
    }
    fn register(&self) -> CreateCommand {
        CreateCommand::new(self.name())
            .description("A key command for testing")
    }

}

impl CommandInfo for KeyCommand {
    fn name(&self) -> &'static str {
        "key"
    }
    
}