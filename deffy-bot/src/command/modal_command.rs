use anyhow::Error;
use deffy_bot_macro::command;
use deffy_bot_utils::ModalBuilder;
use serenity::{
    all::{CommandInteraction, Context, CreateCommand, Permissions},
    async_trait,
};

use crate::command::system::manager::{CommandHandler, CommandInfo};

#[command(cmd = modal,cooldown = 0)]
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

    fn register(&self) -> CreateCommand {
        CreateCommand::new(self.name()).description("test modal").default_member_permissions(Permissions::ADMINISTRATOR)
    }
}


