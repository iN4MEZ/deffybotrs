use anyhow::Error;
use deffy_bot_macro::command;
use deffy_bot_utils::ModalBuilder;
use serenity::{
    all::{CommandInteraction, ComponentInteraction, Context, CreateCommand},
    async_trait,
};

use crate::command::manager::{CommandHandler, CommandInfo};

#[command(cmd = modal)]
pub struct ModalCommand;

#[async_trait]
impl CommandHandler for ModalCommand {
    async fn execute(&self, ctx: Context, interaction: CommandInteraction) -> Result<(), Error> {
        let modal = ModalBuilder::new("myModal", "About you")
            .add_text_input("name", "Name", serenity::all::InputTextStyle::Paragraph)
            .add_text_input(
                "lastname",
                "LastName",
                serenity::all::InputTextStyle::Paragraph,
            )
            .build();

        interaction.create_response(ctx.http, modal).await?;

        Ok(())
    }

    async fn execute_component(
        &self,
        ctx: Context,
        interaction: ComponentInteraction,
    ) -> Result<(), Error> {
        Ok(())
    }

    fn register(&self) -> CreateCommand {
        CreateCommand::new(self.name()).description("test modal")
    }
}


