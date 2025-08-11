use anyhow::Error;
use deffy_bot_macro::command;
use deffy_bot_utils::builder_utils::ModalBuilder;
use serenity::{
    all::{
        CommandInteraction, Context, CreateCommand,
    },
    async_trait,
};

use crate::command::system::manager::{CommandHandler, CommandInfo};

#[command(cmd = verify, cooldown = 10)]
pub struct VerifyCommand;

#[async_trait]
impl CommandHandler for VerifyCommand {
    async fn execute(&self, ctx: Context, interaction: CommandInteraction) -> Result<(), Error> {
        let modal = ModalBuilder::new("verify_patreon", "Verify your email")
            .add_text_input("email", "Email", serenity::all::InputTextStyle::Paragraph)
            .build();

        interaction.create_response(ctx.http, modal).await?;

        Ok(())
    }

    fn register(&self) -> CreateCommand {
        CreateCommand::new(self.name())
            .description("Verify your email")
            .default_member_permissions(serenity::all::Permissions::ADMINISTRATOR)
    }
}