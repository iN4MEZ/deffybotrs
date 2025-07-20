
use serenity::{all::{Context, CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage, Interaction}, async_trait};

use crate::command::command_registry::{CommandHandler, CommandInfo};

pub struct KeyCommand;

#[async_trait]
impl CommandHandler for KeyCommand {
    async fn execute(&self, ctx: Context, data: Interaction) -> Result<(), std::io::Error> {

        let interaction = match data.as_command() {
            Some(c) => c.clone(),
            None => {
                tracing::error!("Interaction is not a command");
                return Err(std::io::Error::new(std::io::ErrorKind::Other, "Interaction is not a command"));
            }
        };

        let ctx_clone = ctx.clone();
        tokio::spawn(async move {
            let content = format!(
                "{}. Key: 100",
                interaction.user.name

            );

             let response = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().content(content).ephemeral(true),
        );

        let result = interaction
            .create_response(ctx_clone.http, response)
            .await;
            if let Err(e) = result {
                tracing::error!("Failed to create response: {}", e);
            }
        });

        Ok(())
        
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