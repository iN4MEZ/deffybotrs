use deffy_bot_utils::ModalBuilder;
use serenity::{all::{CommandInteraction, Context, CreateCommand}, async_trait, Error};

use crate::command::command_registry::{CommandHandler, CommandInfo};

pub struct ModalCommand;

#[async_trait]
impl CommandHandler for ModalCommand {

    async fn execute(&self, ctx: Context, interaction: CommandInteraction) -> Result<(),Error> {

        let modal = ModalBuilder::new("myModal", "About you").
            add_text_input("name", "Name", serenity::all::InputTextStyle::Paragraph)
            .add_text_input("lastname", "LastName", serenity::all::InputTextStyle::Paragraph).build();

            interaction.create_response(ctx.http, modal).await
    }

    fn register(&self) -> CreateCommand {
        CreateCommand::new(self.name())
            .description("test modal")
    }
}

impl CommandInfo for ModalCommand {
    fn name(&self) -> &'static str {
        "modal"
    }
}