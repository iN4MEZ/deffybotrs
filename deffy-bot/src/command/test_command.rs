use anyhow::Error;
use deffy_bot_macro::command;
use serenity::{
    all::{
        CommandInteraction, Context, CreateCommand, Permissions,
    },
    async_trait,
};

use crate::command::system::{interaction_reply::InteractionExt, manager::{CommandHandler, CommandInfo}};

#[command(cmd = test, cooldown = 30)]
pub struct TestCommand;

#[async_trait]
impl CommandHandler for TestCommand {
    async fn execute(&self, ctx: Context, interaction: CommandInteraction) -> Result<(), Error> {

        let msg = deffy_bot_localization::tr!(&interaction.locale, "test");

        let content = format!(
            "Test command executed successfully!\nLocale: {}\nMessage: {}",
            interaction.locale, msg
        );

        interaction.reply(&ctx, content, true).await?;

        Ok(())
    }

    fn register(&self) -> CreateCommand {
        CreateCommand::new(self.name())
            .description("A test command")
            .default_member_permissions(Permissions::ADMINISTRATOR)
    }
}