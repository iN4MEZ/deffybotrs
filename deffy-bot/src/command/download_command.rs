use anyhow::Error;
use deffy_bot_macro::command;
use serenity::{all::{CommandInteraction, ComponentInteraction, Context, CreateCommand}, async_trait};

use crate::command::manager::CommandHandler;

#[command(cmd = dl)]
pub struct DownloadCommand;

#[async_trait]
impl CommandHandler for DownloadCommand {
    async fn execute(&self, _ctx: Context, _data: CommandInteraction) -> Result<(), Error> {
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
        CreateCommand::new("download")
            .description("Download a file from the internet")
    }
}