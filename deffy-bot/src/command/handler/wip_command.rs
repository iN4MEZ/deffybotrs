use anyhow::Error;
use deffy_bot_macro::command;
use deffy_bot_utils::wip_database::{WipDatabase, WipEntry};
use serenity::{all::*, async_trait};

use crate::command::system::{
    interaction_reply::InteractionExt,
    manager::{CommandHandler, CommandInfo},
};

// ===== /wip_create =====
#[command(cmd = wip_create, cooldown = 0)]
pub struct WipCreateCommand;

#[async_trait]
impl CommandHandler for WipCreateCommand {
    async fn execute(&self, ctx: Context, interaction: CommandInteraction) -> Result<(), Error> {
        let title = interaction
            .data
            .options
            .get(0)
            .and_then(|opt| opt.value.as_str())
            .unwrap_or("Untitled WIP");

        // รับ percent ตรง ๆ
        let percent = interaction
            .data
            .options
            .get(1)
            .and_then(|opt| opt.value.as_i64())
            .unwrap_or(0) as u8;

        let image = interaction.data.options.get(2)
            .and_then(|opt| opt.value.as_str())
            .unwrap_or("https://static.wikia.nocookie.net/wutheringwaves/images/e/ee/Cartethyia_Full_Sprite.png");

        let description = interaction
            .data
            .options
            .get(3)
            .and_then(|opt| opt.value.as_str())
            .unwrap_or("No description provided");

        // แปลง percent -> state อัตโนมัติ
        let state = match percent {
            0..=9 => 1,
            10..=29 => 2,
            30..=49 => 3,
            50..=79 => 4,
            80..=94 => 5,
            _ => 6,
        };

        let msg = interaction
            .channel_id
            .send_message(
                &ctx.http,
                CreateMessage::new().embed(make_progress_embed(
                    title,
                    image,
                    description,
                    percent,
                    18,
                )),
            )
            .await?;

        // บันทึก DB ด้วย state เดิม
        let entry = WipEntry {
            title: title.to_string(),
            channel_id: interaction.channel_id.get(),
            message_id: msg.id.get(),
            image: image.to_string(),             // Save the image URL
            description: description.to_string(), // Save the description
            state,
        };
        WipDatabase::create_wip(entry).await?;

        interaction
            .reply(
                &ctx,
                format!(
                    "✅ WIP `{}` created at {}% (state {})",
                    title, percent, state
                ),
                true,
            )
            .await?;
        Ok(())
    }

    fn register(&self) -> CreateCommand {
        CreateCommand::new(self.name())
            .description("Create a new WIP progress tracker with percent (0-100)")
            .default_member_permissions(Permissions::ADMINISTRATOR)
            .add_option(
                CreateCommandOption::new(CommandOptionType::String, "title", "Title of the WIP")
                    .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "percent",
                    "Progress percent (0-100)",
                )
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::String, "thumbnail", "url")
                    .required(true),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::String, "description", "des (aug)")
                    .required(true),
            )
    }
}

// ===== /wip_update =====
#[command(cmd = wip_update, cooldown = 0)]
pub struct WipUpdateCommand;

#[async_trait]
impl CommandHandler for WipUpdateCommand {
    async fn execute(&self, ctx: Context, interaction: CommandInteraction) -> Result<(), Error> {
        let title = interaction
            .data
            .options
            .get(0)
            .and_then(|opt| opt.value.as_str())
            .unwrap_or("Untitled WIP");

        // รับ percent
        let percent = interaction
            .data
            .options
            .get(1)
            .and_then(|opt| opt.value.as_i64())
            .unwrap_or(0) as u8;

        // แปลง percent -> state
        let state = match percent {
            0..=9 => 1,
            10..=29 => 2,
            30..=49 => 3,
            50..=79 => 4,
            80..=94 => 5,
            _ => 6,
        };

        if let Some(wip) = WipDatabase::get_wip(title).await? {
            let embed = make_progress_embed(&wip.title, &wip.image, &wip.description, percent, 18);

            ChannelId::new(wip.channel_id)
                .edit_message(
                    &ctx.http,
                    MessageId::new(wip.message_id),
                    EditMessage::new().embed(embed),
                )
                .await?;

            WipDatabase::update_wip(WipEntry {
                title: wip.title.clone(),
                channel_id: wip.channel_id,
                message_id: wip.message_id,
                image: wip.image.clone(),
                description: wip.description.clone(),
                state,
            })
            .await?;

            interaction
                .reply(
                    &ctx,
                    format!(
                        "🔄 Updated `{}` to {}% (state {})",
                        wip.title, percent, state
                    ),
                    true,
                )
                .await?;
        } else {
            interaction
                .reply(&ctx, format!("❌ WIP `{}` not found", title), true)
                .await?;
        }

        Ok(())
    }

    fn register(&self) -> CreateCommand {
        CreateCommand::new(self.name())
            .description("Update WIP progress by percent (0-100)")
            .default_member_permissions(Permissions::ADMINISTRATOR)
            .add_option(
                CreateCommandOption::new(CommandOptionType::String, "title", "Title of the WIP")
                    .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "percent",
                    "Progress percent (0-100)",
                )
                .required(true),
            )
    }
}

// ===== /wip_remove =====
#[command(cmd = wip_remove, cooldown = 0)]
pub struct WipRemoveCommand;
#[async_trait]
impl CommandHandler for WipRemoveCommand {
    async fn execute(&self, ctx: Context, interaction: CommandInteraction) -> Result<(), Error> {
        let title = interaction
            .data
            .options
            .get(0)
            .and_then(|opt| opt.value.as_str())
            .unwrap_or("Untitled WIP");

        if let Some(wip) = WipDatabase::get_wip(title).await? {
            ChannelId::new(wip.channel_id)
                .delete_message(&ctx.http, MessageId::new(wip.message_id))
                .await?;

            WipDatabase::remove_wip(title).await?;
            interaction
                .reply(
                    &ctx,
                    format!("✅ WIP `{}` removed successfully", title),
                    true,
                )
                .await?;
        }

        Ok(())
    }

    fn register(&self) -> CreateCommand {
        CreateCommand::new(self.name())
            .description("Remove a WIP progress tracker by title")
            .default_member_permissions(Permissions::ADMINISTRATOR)
            .add_option(
                CreateCommandOption::new(CommandOptionType::String, "title", "Title of the WIP")
                    .required(true),
            )
    }
}

fn make_progress_embed(
    title: &str,
    image: &str,
    description: &str,
    percent: u8,
    width: usize,
) -> CreateEmbed {
    let bar = progress_bar(percent, width);
    let percent_text = format!("{}%", percent);
    let status = status_from_percent(percent);
    let color = colour_from_percent(percent);

    CreateEmbed::default()
        .title(format!("{} – {}", title, status))
        .description(description)
        .thumbnail(image)
        .colour(color)
        .field("Progress", format!("`{}`\n{}", percent_text, bar), false)
        .timestamp(chrono::Utc::now())
        .footer(CreateEmbedFooter::new("Patreon Work in Progress"))
}

fn progress_bar(percent: u8, width: usize) -> String {
    let filled = ((percent as f64 / 100.0 * width as f64).round() as usize).min(width);
    let empty = width - filled;
    format!("`[{}{}]`", "█".repeat(filled), "░".repeat(empty))
}

fn status_from_percent(percent: u8) -> String {
    match percent {
        0..=9 => "📦 Collect Resource For Scene".into(),
        10..=29 => "🔧 Rigging".into(),
        30..=49 => "🎬 Animating".into(),
        50..=59 => "✨ Final Animating".into(),
        60..=79 => "🖥 Rendering".into(),
        80..=94 => "🎬 Editing".into(),
        95..=100 => "✅ Completed".into(),
        _ => "❓ Unknown".into(),
    }
}

fn colour_from_percent(percent: u8) -> Colour {
    if percent < 25 {
        Colour::from_rgb(0xE7, 0x4C, 0x3C)
    } else if percent < 50 {
        Colour::from_rgb(0xF1, 0xC4, 0x0F)
    } else if percent < 75 {
        Colour::from_rgb(0x2E, 0xCC, 0x71)
    } else if percent < 100 {
        Colour::from_rgb(0x27, 0xAE, 0x60)
    } else {
        Colour::from_rgb(0x9B, 0x59, 0xB6)
    }
}
