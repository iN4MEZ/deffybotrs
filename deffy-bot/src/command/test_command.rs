
use anyhow::Error;
use deffy_bot_macro::command;
use serenity::{
    all::{CommandInteraction, Context, CreateCommand, CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage},
    async_trait
};

use crate::command::manager::{CommandHandler, CommandInfo};

#[command(cmd = test)]
pub struct TestCommand;

#[async_trait]
impl CommandHandler for TestCommand {
    async fn execute(&self, ctx: Context, interaction: CommandInteraction) -> Result<(), Error> {
        let content = format!(
            "Hello, {} This is a test command response.",
            interaction.user.name
        );

         let response = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new().content(content),
    );

    let result = interaction
        .create_response(ctx.http, response)
        .await?;

    Ok(result)

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
