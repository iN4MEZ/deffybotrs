use deffy_bot_macro::command;
use serenity::{all::{CommandInteraction, Context, CreateCommand}, async_trait, Error};

use crate::command::manager::CommandHandler;

#[command(cmd = dl)]
pub struct DownloadCommand;

#[async_trait]
impl CommandHandler for DownloadCommand {
    async fn execute(&self, _ctx: Context, _data: CommandInteraction) -> Result<(), Error> {
        Ok(())
    }

    fn register(&self) -> CreateCommand {
        CreateCommand::new("download")
            .description("Download a file from the internet")
    }
}