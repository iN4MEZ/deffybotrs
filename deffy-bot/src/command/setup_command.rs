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
            "setlogchannel" => {
                handle_set_logging_channel(&interaction).await?;
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
            .description("A setup command for admin")
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
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "setlogchannel",
                    "set logging channel",
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::Channel,
                        "logchannel",
                        "what channel",
                    )
                    .required(true),
                ),
            )
    }
}

pub async fn handle_set_role_verify(interaction: &CommandInteraction) -> Result<(), Error> {
    let Some(CommandDataOptionValue::Role(role_id)) = get_sub_option_value(interaction, "role")
    else {
        return Err(anyhow::anyhow!("No role ID found"));
    };

    if let Some(sv_id) = interaction.guild_id {
        return DiscordServerDatabaseManager::set_verify_roles(sv_id.get(), role_id.get()).await;
    }

    Err(anyhow::anyhow!("Guild ID not found"))
}

pub async fn handle_set_logging_channel(interaction: &CommandInteraction) -> Result<(), Error> {
    let Some(CommandDataOptionValue::Channel(channel_id)) =
        get_sub_option_value(interaction, "logchannel")
    else {
        return Err(anyhow::anyhow!("No channel ID found"));
    };

    if let Some(sv_id) = interaction.guild_id {
        return DiscordServerDatabaseManager::set_logging_channel(sv_id.get(), channel_id.get())
            .await;
    }
    Err(anyhow::anyhow!("Guild ID not found"))
}

pub fn get_sub_option_value<'a>(
    interaction: &'a CommandInteraction,
    option_name: &str,
) -> Option<&'a CommandDataOptionValue> {
    if let Some(CommandDataOptionValue::SubCommand(options)) =
        interaction.data.options.get(0).map(|opt| &opt.value)
    {
        options
            .iter()
            .find(|opt| opt.name == option_name)
            .map(|opt| &opt.value)
    } else {
        None
    }
}
