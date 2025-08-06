use anyhow::Error;
use deffy_bot_macro::command;
use deffy_bot_utils::DiscordServerDatabaseManager;
use serenity::{
    all::{
        CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
        CreateCommandOption, Permissions,
    },
    async_trait,
};

use crate::command::system::{
    interaction_reply::InteractionExt,
    manager::{CommandHandler, CommandInfo},
};

#[command(cmd = setup, cooldown = 0)]
pub struct SetupCommand;

#[async_trait]
impl CommandHandler for SetupCommand {
    async fn execute(&self, ctx: Context, interaction: CommandInteraction) -> Result<(), Error> {
        let subcommand = interaction
            .data
            .options
            .first()
            .and_then(|opt| Some(opt.name.as_str()))
            .ok_or_else(|| anyhow::anyhow!("No subcommand provided"))?;

        match subcommand {
            "role_verify" => {
                handle_set_role_verify(&interaction).await?;
                tracing::debug!("set the verify role")
            }
            _ => {}
        }

        let content = format!(
            "Hello, {} This is a test command response.",
            interaction.user.name
        );

        interaction.reply(&ctx, content, true).await?;

        Ok(())
    }

    fn register(&self) -> CreateCommand {
        CreateCommand::new(self.name())
            .description("A test command")
            .default_member_permissions(Permissions::ADMINISTRATOR)
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "role_verify",
                    "set verify role",
                )
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::Role, "role", "select verify role")
                        .required(true),
                ),
            )
    }
}

pub async fn handle_set_role_verify(interaction: &CommandInteraction) -> Result<(), Error> {
    if let Some(CommandDataOptionValue::SubCommand(options)) =
        interaction.data.options.get(0).map(|opt| &opt.value)
    {
        if let Some(role_id) = options
            .iter()
            .find(|opt| opt.name == "role")
            .and_then(|opt| match &opt.value {
                CommandDataOptionValue::Role(role_id) => Some(role_id),
                _ => None,
            })
        {
            if let Some(sv_id) = interaction.guild_id {
                return DiscordServerDatabaseManager::set_verify_roles(sv_id.get(),role_id.get()).await;
            }

        }
    }
    return Err(anyhow::anyhow!("No role id found in option"))

}