use serenity::{
    all::{
        CommandInteraction, Context, CreateAttachment, CreateCommand, CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage, EditInteractionResponse, EditProfile, Permissions
    },
    async_trait, Error,
};

use crate::command::command_registry::{CommandHandler, CommandInfo};

pub struct ProfileCommand;

#[async_trait]
impl CommandHandler for ProfileCommand {
    async fn execute(&self, ctx: Context, interaction: CommandInteraction) -> Result<(), Error> {

         interaction.create_response(&ctx.http, CreateInteractionResponse::Defer(CreateInteractionResponseMessage::new())).await?;

            if let Some(input) = interaction.data.options.get(0) {
                match input.value.as_str() {
                    Some("p") => {
                        tracing::info!("Profile command executed with type: profile");

                        let att = create_att(&interaction).await.expect("Not found Interaction");

                        if let Err(err) = ctx
                            .http
                            .edit_profile(&EditProfile::new().avatar(&att))
                            .await
                        {
                            tracing::error!("Failed to update avatar: {:?}", err);
                        } else {
                            tracing::info!("Avatar updated successfully!");
                        }
                    }
                    Some("b") => {
                        tracing::info!("Profile command executed with type: banner");

                        let att = create_att(&interaction).await.unwrap();

                        if let Err(err) = ctx
                            .http
                            .edit_profile(&EditProfile::new().banner(&att))
                            .await
                        {
                            tracing::error!("Failed to update banner: {:?}", err);
                        } else {
                            tracing::info!("Banner updated successfully!");
                        }
                    }
                    _ => {
                        tracing::warn!("Unknown profile command type");
                    }
                }
            } else {
                tracing::warn!("ProfileCommand executed without input");
            }
            let content = format!(
                "All Profile information retrieved successfully."
            );

            interaction
                .edit_response(
                    ctx.http,
                    EditInteractionResponse::new().content(content),
                )
                .await?;
            Ok(())
    }

    fn register(&self) -> CreateCommand {
        CreateCommand::new(self.name())
            .description("A profile command for testing")
            .add_option(
                CreateCommandOption::new(
                    serenity::all::CommandOptionType::String,
                    "type",
                    "An input string for profile command",
                )
                .required(true)
                .add_string_choice("profile", "p")
                .add_string_choice("banner", "b"),
            )
            .add_option(
                CreateCommandOption::new(
                    serenity::all::CommandOptionType::Attachment,
                    "attachment",
                    "file",
                )
                .required(true),
            )
            .default_member_permissions(Permissions::ADMINISTRATOR)
    }
}

impl CommandInfo for ProfileCommand {
    fn name(&self) -> &'static str {
        "profile"
    }
}

pub async fn create_att(interaction: &CommandInteraction) -> Option<CreateAttachment> {
    if let Some(attachment_opt) = interaction.data.options.get(1) {
        if let Some(file) = interaction
            .data
            .resolved
            .attachments
            .get(&attachment_opt.value.as_attachment_id().unwrap_or_default())
        {
            if file.content_type != Some("image/gif".to_string())
                && file.content_type != Some("image/jpeg".to_string())
                && file.content_type != Some("image/png".to_string())
            {
                tracing::warn!("Invalid attachment type: {:?}", file.content_type);
                return None;
            }

            let client = reqwest::Client::new();
            let response = client
                .get(&file.url)
                .send()
                .await
                .expect("Failed to download file");

            let data = response
                .bytes()
                .await
                .expect("Failed to read response bytes");

            tracing::info!("File downloaded successfully: {}", data.len());

            return Some(CreateAttachment::bytes(data.to_vec(), "avatar.png"));
        } else {
            tracing::warn!("No valid attachment provided");
        }
    } else {
        tracing::warn!("No attachment option found");
    }

    None
}