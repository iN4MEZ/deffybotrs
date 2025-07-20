
use serenity::{
    all::{Context, CreateCommand, CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage, Interaction},
    async_trait,
};

use crate::command::command_registry::{CommandHandler, CommandInfo};

pub struct TestCommand;

#[async_trait]
impl CommandHandler for TestCommand {
    async fn execute(&self, ctx: Context, data: Interaction) -> Result<(), std::io::Error> {
        tracing::info!("TestCommand executed");

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
                "Hello, {} This is a test command response.",
                interaction.user.name
            );

             let response = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().content(content),
        );

        let _ = interaction
            .create_response(ctx_clone.http, response)
            .await;
        
        });

        Ok(())
    }
    fn register(&self) -> CreateCommand {
        CreateCommand::new(self.name())
            .description("A test command")
            .add_option(CreateCommandOption::new(
                serenity::all::CommandOptionType::String,
                "input",
                "An input string for testing",
            ).required(true))
    }
}

impl CommandInfo for TestCommand {
    fn name(&self) -> &'static str {
        "test"
    }
}
