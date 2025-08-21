use anyhow::{Error, Ok};
use deffy_bot_encryption::EncrytionHelper;
use deffy_bot_macro::command;
use serenity::{
    all::{
        CommandInteraction, Context, CreateCommand, CreateEmbed, Permissions
    },
    async_trait,
};

use crate::command::system::{interaction_reply::InteractionExt, manager::{CommandHandler, CommandInfo}};

#[command(cmd = key, cooldown = 0)]
pub struct KeyCommand;

#[async_trait]
impl CommandHandler for KeyCommand {
    async fn execute(&self, ctx: Context, interaction: CommandInteraction) -> Result<(), Error> {
        let enc = EncrytionHelper::encrypt("hello");

        let content = format!("{}. Key: {}", interaction.user.name, enc);

        let embed = CreateEmbed::new().title(content);

        interaction.reply_embed(&ctx, embed, true).await?;

        Ok(())

    }

    fn register(&self) -> CreateCommand {
        CreateCommand::new(self.name()).description("A key command for testing").default_member_permissions(Permissions::ADMINISTRATOR)
    }
}
