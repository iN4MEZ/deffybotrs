use std::{
    collections::HashSet,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Error, Ok};
use chrono::Utc;
use deffy_bot_macro::command;
use once_cell::sync::Lazy;
use serenity::{
    all::{
        ButtonStyle, CommandInteraction, CommandOptionType, ComponentInteractionCollector, ComponentInteractionDataKind, Context, CreateActionRow, CreateButton, CreateCommand, CreateCommandOption, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse, CreateInteractionResponseFollowup, CreateInteractionResponseMessage, CreateMessage, CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption, EditMessage, Permissions, UserId
    },
    async_trait,
    futures::{future::join_all, StreamExt},
};
use tokio::{sync::Mutex, time::sleep};

use crate::command::system::{
    interaction_reply::InteractionExt,
    manager::{CommandHandler, CommandInfo},
};

pub static ACTIVE_BANS: Lazy<Mutex<HashSet<UserId>>> = Lazy::new(|| Mutex::new(HashSet::new()));

#[command(cmd = moderate, cooldown = 0)]
pub struct ModerateCommand;

#[async_trait]
impl CommandHandler for ModerateCommand {
    async fn execute(&self, ctx: Context, interaction: CommandInteraction) -> Result<(), Error> {
        let subcommand = interaction
            .data
            .options
            .first()
            .and_then(|opt| Some(opt.name.as_str()))
            .ok_or_else(|| anyhow::anyhow!("No subcommand provided"))?;

        let content = format!("Opening Ban menu.");

        interaction.reply(&ctx, content, true).await?;

        match subcommand {
            "ban" => {
                tokio::spawn(async move {
                    if let Err(err) = send_ban_menu(&ctx, &interaction).await {
                        let follow_up = CreateInteractionResponseFollowup::new()
                            .content(format!("error: {}", err))
                            .ephemeral(true);

                        let _ = interaction.create_followup(&ctx, follow_up).await;
                    }
                });
            }
            _ => {}
        }

        Ok(())
    }

    fn register(&self) -> CreateCommand {
        CreateCommand::new(self.name())
            .description("A moderator command for admin")
            .default_member_permissions(Permissions::ADMINISTRATOR)
            .add_option(CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "ban",
                "ban user",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "timeout",
                "timeout user",
            ))
    }
}

async fn send_ban_menu(ctx: &Context, interaction: &CommandInteraction) -> Result<(), Error> {
    let run_owner_id = &interaction.user.id;

    {
        let mut active = ACTIVE_BANS.lock().await;

        if active.contains(run_owner_id) {
            return Err(anyhow::anyhow!("You already run ban command!"));
        }
        // เพิ่มเข้าไปเพื่อกันซ้ำ
        active.insert(*run_owner_id);
        tracing::debug!("user{} added", &run_owner_id);
    }

    let custom_id = format!("banuser:{}", &run_owner_id);

    let confirm_btn_owner_custom_id = format!("confirmbanbtn:{}", &run_owner_id);
    let cancel_btn_owner_custom_id = format!("cancelbanbtn:{}", &run_owner_id);

    let channel_id = interaction.channel_id;

    let select_menu = create_select_menu(
        &custom_id,
        "please select users",
        CreateSelectMenuKind::User {
            default_users: Some(vec![]),
        },
    );

    let ban_rule_id_select_menu = CreateActionRow::SelectMenu(
        CreateSelectMenu::new("banruleid", CreateSelectMenuKind::String {
            options: vec![
                CreateSelectMenuOption::new("Rule 1", "rule1"),
                CreateSelectMenuOption::new("Rule 2", "rule2"),
                CreateSelectMenuOption::new("Rule 3", "rule3"),
            ],
        })
        .placeholder("Please select a ban rule")
        .min_values(1)
        .max_values(1),
    );

    let action_row_btn = create_btn_ban_action_row(
        true,
        &confirm_btn_owner_custom_id,
        &cancel_btn_owner_custom_id,
    );

    let components = vec![select_menu,ban_rule_id_select_menu, action_row_btn];

    let timeout = Duration::from_secs(60);

    // คำนวณ expiry เป็น unix seconds (สำหรับ Discord timestamp)
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();
    let expiry_secs = now_secs + timeout.as_secs();

    let message_embed = CreateEmbed::new()
        .color(0xffa800)
        .title("MODERATE BAN MENU")
        .description("Select *user's* to ban from the server.")
        .footer(CreateEmbedFooter::new(format!("Run by: {}", interaction.user.name)))
        .thumbnail("https://static.wikia.nocookie.net/zenless-zone-zero/images/c/ce/Base_Bangboo_Portrait.png")
        .timestamp(Utc::now());

    let mut msg = channel_id
        .send_message(
            &ctx.http,
            CreateMessage::new()
                .content(format!("Timeout: <t:{}:R>", expiry_secs))
                .embed(message_embed)
                .components(components),
        )
        .await?;

    let ctx_clone = ctx.clone();
    let mut msg_clone = msg.clone();
    let run_owner_id_clone = run_owner_id.clone();

    // ใช้ tokio::spawn เพื่อรอ timeout แล้วแก้ข้อความ
    tokio::spawn(async move {
        sleep(timeout).await;

        let mut active = ACTIVE_BANS.lock().await;

        active.remove(&run_owner_id_clone);

        tracing::debug!("user:{} removed", &run_owner_id_clone);

        msg_clone
            .edit(
                &ctx_clone.http,
                EditMessage::new()
                    .content(format!(
                        "⏳ หมดเวลาเลือกแล้ว (หมดเวลา {})",
                        format!("<t:{}:R>", expiry_secs)
                    ))
                    .components(vec![])
                    .embeds(vec![]),
            )
            .await
            .ok();
    });

    let custom_id_clone = custom_id.clone();

    let mut collector = ComponentInteractionCollector::new(&ctx.shard)
        .filter(move |mci| mci.data.custom_id == custom_id)
        .timeout(timeout)
        .stream();

    while let Some(interaction) = collector.next().await {
        if let ComponentInteractionDataKind::UserSelect { values } = &interaction.data.kind {
            // ป้องกันคนอื่นมากด select
            if interaction.user.id != run_owner_id_clone {
                // ตอบกลับว่าไม่ใช่เจ้าของ
                let _ = interaction
                    .create_response(
                        &ctx.http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .content("You are not the owner of this command!")
                                .ephemeral(true),
                        ),
                    )
                    .await;
                continue;
            }

            let names: Vec<String> = join_all(values.iter().map(|uid| ctx.http.get_user(*uid)))
                .await
                .into_iter()
                .filter_map(Result::ok)
                .map(|u| u.name.clone())
                .collect();

            let responded_at_secs = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            let select_menu = create_select_menu(
                &custom_id_clone,
                "please select users",
                CreateSelectMenuKind::User {
                    default_users: Some(Vec::new()),
                },
            );

            let action_row_btn = create_btn_ban_action_row(
                !(names.len() > 0),
                &confirm_btn_owner_custom_id,
                &cancel_btn_owner_custom_id,
            );

            let components = vec![select_menu, action_row_btn];

            msg.edit(&ctx.http, EditMessage::new().components(components))
                .await?;

            let result = interaction
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content(format!(
                                "คุณเลือก: {}\n(ตอบเมื่อ {})",
                                names.join(", "),
                                format!("<t:{}:R>", responded_at_secs)
                            ))
                            .ephemeral(true),
                    ),
                )
                .await;

            if let Err(err) = result {
                tracing::error!("{}", err);
            }
        }
    }

    Ok(())
}

fn create_btn_ban_action_row(
    is_disabled: bool,
    custom_confirm_id: &str,
    custom_cancel_id: &str,
) -> CreateActionRow {
    let confirm_action_btn = CreateButton::new(custom_confirm_id)
        .style(ButtonStyle::Success)
        .label("CONFIRM BAN")
        .disabled(is_disabled);

    let cancel_action_btn = CreateButton::new(custom_cancel_id)
        .style(ButtonStyle::Danger)
        .label("CANCEL");

    let action_row_btn = CreateActionRow::Buttons(vec![confirm_action_btn, cancel_action_btn]);

    action_row_btn
}

fn create_select_menu(
    custom_id: &str,
    placeholder: &str,
    kind: CreateSelectMenuKind,
) -> CreateActionRow {
    let select_menu = CreateActionRow::SelectMenu(
        CreateSelectMenu::new(custom_id, kind)
            .placeholder(placeholder)
            .min_values(1)
            .max_values(10),
    );
    select_menu
}
