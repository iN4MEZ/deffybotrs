use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Error, Ok};
use chrono::Utc;
use deffy_bot_macro::{command, event};
use once_cell::sync::Lazy;
use serenity::{
    all::{
        ButtonStyle, CommandInteraction, CommandOptionType, ComponentInteractionCollector,
        ComponentInteractionDataKind, Context, CreateActionRow, CreateButton, CreateCommand,
        CreateCommandOption, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
        CreateInteractionResponseFollowup, CreateInteractionResponseMessage, CreateMessage,
        CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption, EditMessage, Message,
        MessageId, Permissions, UserId,
    },
    async_trait,
    futures::StreamExt,
};
use tokio::{
    sync::{
        Mutex,
        mpsc::{self, UnboundedSender},
    },
    time::sleep,
};

use crate::{
    command::system::manager::{CommandHandler, CommandInfo},
    event::manager::EventData,
};

#[derive(Debug, Clone)]
struct BanUserTarget {
    admin_id: UserId,
    users: Vec<UserId>,
}

static BANS: Lazy<Arc<Mutex<Vec<BanUserTarget>>>> = Lazy::new(|| Arc::new(Mutex::new(Vec::new())));

static ACTIVE_BANS_MENU: Lazy<Mutex<HashMap<UserId, MessageId>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

static COLLECTOR_STOPS: Lazy<Mutex<HashMap<UserId, UnboundedSender<()>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

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

        interaction.defer_ephemeral(&ctx.http).await?;

        match subcommand {
            "ban" => {
                tokio::spawn(async move {
                    let result = send_ban_menu(&ctx, &interaction).await;

                    if let Err(err) = &result {
                        if err.to_string().contains("You already run ban command!") {
                            let follow_up = CreateInteractionResponseFollowup::new()
                                .content("You already run ban command!")
                                .ephemeral(true);

                            let result = interaction.create_followup(&ctx, follow_up).await;

                            if let Err(err) = result {
                                tracing::error!("Failed to send follow-up message: {:?}", err);
                            }
                        }
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
        let active = ACTIVE_BANS_MENU.lock().await;

        if active.contains_key(run_owner_id) {
            return Err(anyhow::anyhow!("You already run ban command!"));
        }
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
        CreateSelectMenu::new(
            "banruleid",
            CreateSelectMenuKind::String {
                options: vec![
                    CreateSelectMenuOption::new("Rule 1", "rule1"),
                    CreateSelectMenuOption::new("Rule 2", "rule2"),
                    CreateSelectMenuOption::new("Rule 3", "rule3"),
                ],
            },
        )
        .placeholder("Please select a ban rule")
        .min_values(1)
        .max_values(1),
    );

    let action_row_btn = create_btn_ban_action_row(
        true,
        &confirm_btn_owner_custom_id,
        &cancel_btn_owner_custom_id,
    );

    let components = vec![select_menu, ban_rule_id_select_menu, action_row_btn];

    let timeout = Duration::from_secs(120);

    // คำนวณ expiry เป็น unix seconds (สำหรับ Discord timestamp)
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();
    let expiry_secs = now_secs + timeout.as_secs();

    let message_embed = CreateEmbed::new()
        .color(0xffa800)
        .title("MODERATOR BAN MENU")
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

    {
        let mut active = ACTIVE_BANS_MENU.lock().await;

        active.insert(run_owner_id.clone(), msg.id);
        tracing::debug!("user:{} added", &run_owner_id);
    }

    let ctx_clone = ctx.clone();
    let msg_clone = msg.clone();
    let run_owner_id_clone = run_owner_id.clone();
    let custom_id_clone = custom_id.clone();

    let (stop_tx, stop_rx) = mpsc::unbounded_channel();

    COLLECTOR_STOPS
        .lock()
        .await
        .insert(run_owner_id_clone, stop_tx);

    run_ban_collector_timeout_awaiter(
        timeout,
        &ctx_clone,
        run_owner_id_clone,
        msg_clone,
        interaction.clone(),
        stop_rx,
    )
    .await;

    let mut collector = ComponentInteractionCollector::new(&ctx.shard)
        .filter(move |mci| mci.data.custom_id == custom_id)
        .timeout(timeout)
        .stream();

    while let Some(interaction) = collector.next().await {
        if let ComponentInteractionDataKind::UserSelect { values } = &interaction.data.kind {
            // ป้องกันคนอื่นมากด select
            if interaction.user.id != run_owner_id_clone {
                interaction
                    .create_response(
                        &ctx.http,CreateInteractionResponse::Acknowledge
                    )
                    .await?;
                continue;
            }

            let user_count = values.len();

            let select_menu = create_select_menu(
                &custom_id_clone,
                "please select users",
                CreateSelectMenuKind::User {
                    default_users: Some(Vec::new()),
                },
            );

            let action_row_btn = create_btn_ban_action_row(
                !(user_count > 0),
                &confirm_btn_owner_custom_id,
                &cancel_btn_owner_custom_id,
            );

            let components = vec![select_menu, action_row_btn];

            msg.edit(&ctx.http, EditMessage::new().components(components))
                .await?;
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

    let reason_action_btn = CreateButton::new("banreason")
        .style(ButtonStyle::Secondary)
        .label("REASON")
        .disabled(is_disabled);

    let attactment_action_btn = CreateButton::new("banattachment")
        .style(ButtonStyle::Primary)
        .label("ATTACHMENT")
        .disabled(is_disabled);

    let action_row_btn = CreateActionRow::Buttons(vec![
        confirm_action_btn,
        reason_action_btn,
        attactment_action_btn,
        cancel_action_btn,
    ]);

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
            .min_values(0)
            .max_values(10),
    );
    select_menu
}

async fn run_ban_collector_timeout_awaiter(
    timeout: Duration,
    ctx: &Context,
    run_owner_id: UserId,
    msg: Message,
    interaction: CommandInteraction,
    mut stop_rx: mpsc::UnboundedReceiver<()>, // เพิ่ม receiver สำหรับหยุด
) {
    let ctx_clone = ctx.clone();
    let mut msg_clone = msg.clone();
    let run_owner_id_clone = run_owner_id.clone();

    tokio::spawn(async move {
        tokio::select! {
            _ = sleep(timeout) => {
                tracing::debug!("timeout for {}", &run_owner_id_clone);
            }
            _ = stop_rx.recv() => {
                tracing::debug!("force stop for {}", &run_owner_id_clone);
            }
        }

        let mut active = ACTIVE_BANS_MENU.lock().await;
        active.remove(&run_owner_id_clone);
        COLLECTOR_STOPS.lock().await.remove(&run_owner_id_clone);
        tracing::debug!("user:{} removed", &run_owner_id_clone);

        interaction
            .create_followup(
                &&ctx_clone,
                CreateInteractionResponseFollowup::new()
                    .content("Ban selection has ended.")
                    .ephemeral(true),
            )
            .await
            .ok();

        msg_clone
            .edit(
                &ctx_clone.http,
                EditMessage::new()
                    .content("หมดเวลาเลือกแล้ว")
                    .components(vec![])
                    .embeds(vec![]),
            )
            .await
            .ok();
    });
}

#[event(e = interaction_create)]
async fn on_interaction(ctx: Context, data: EventData) {
    if let EventData::Interaction(interaction) = data {
        if let Some(sm) = interaction.as_message_component() {
            let user_interact_id = sm.user.id;

            let active = ACTIVE_BANS_MENU.lock().await;

            if active.contains_key(&user_interact_id) {
                let select_menu_id = format!("banuser:{}", user_interact_id);

                match sm.data.custom_id.as_str() {
                    id if id.starts_with("confirmbanbtn:") => {
                        if let Some(owner_id_str) = id.strip_prefix("confirmbanbtn:") {
                            if let Some(owner_id) = owner_id_str.parse::<u64>().ok() {
                                if sm.user.id.get() != owner_id {
                                    tracing::warn!(
                                        "User {} tried to confirm ban but is not the owner",
                                        user_interact_id
                                    );
                                    return;
                                }
                                if let Some(message_id) = active.get(&user_interact_id).cloned() {
                                    if let Err(err) =
                                        sm.channel_id.delete_message(&ctx.http, message_id).await
                                    {
                                        tracing::error!("Failed to delete message: {:?}", err);
                                    }
                                } else {
                                    tracing::warn!(
                                        "No active ban found for user: {}",
                                        user_interact_id
                                    );
                                }

                                let bans = BANS.lock().await;

                                for user in bans.iter() {
                                    if user.admin_id == user_interact_id {
                                        for target_user in &user.users {
                                            // test with dm
                                            let dm = target_user.create_dm_channel(&ctx.http).await;
                                            if dm.is_ok() {
                                                let dm_channel = dm.unwrap();
                                                let content = format!(
                                                    "You have been banned by <@{}>.\nReason: {}",
                                                    user_interact_id, sm.data.custom_id
                                                );

                                                if let Some(stop_tx) = COLLECTOR_STOPS
                                                    .lock()
                                                    .await
                                                    .remove(&user_interact_id)
                                                {
                                                    let _ = stop_tx.send(()); // ส่งสัญญาณหยุด
                                                }

                                                let result = dm_channel
                                                    .send_message(
                                                        &ctx.http,
                                                        CreateMessage::new().content(content),
                                                    )
                                                    .await;

                                                if let Err(e) = result {
                                                    tracing::error!(
                                                        "Failed to send DM to {}: {:?}",
                                                        target_user,
                                                        e
                                                    );
                                                }
                                            }
                                        }
                                    }

                                    let response = sm
                                        .create_response(
                                            &ctx.http,
                                            CreateInteractionResponse::Message(
                                                CreateInteractionResponseMessage::new()
                                                    .content("Ban confirmed!")
                                                    .ephemeral(true),
                                            ),
                                        )
                                        .await;

                                    if let Err(e) = response {
                                        tracing::error!(
                                            "Ban Confirm: Failed to send response: {:?}",
                                            e
                                        );
                                    }
                                }
                            }
                        }
                    }

                    id if id == select_menu_id => {
                        if let ComponentInteractionDataKind::UserSelect { values } = &sm.data.kind {
                            tracing::debug!("User selected: {:?}", values);

                            handle_select(user_interact_id, values.clone()).await;

                            // let names: Vec<String> =
                            //     join_all(values.iter().map(|uid| ctx.http.get_user(*uid)))
                            //         .await
                            //         .into_iter()
                            //         .filter_map(Result::ok)
                            //         .map(|u| u.name.clone())
                            //         .collect();

                            let response = sm
                                .create_response(&ctx.http, CreateInteractionResponse::Acknowledge)
                                .await;
                            if let Err(e) = response {
                                tracing::error!("Select Menu: Failed to send response: {:?}", e);
                            }
                        }
                    }

                    id if id.starts_with("cancelbanbtn:") => {
                        if let Some(owner_id_str) = id.strip_prefix("cancelbanbtn:") {
                            if let Some(owner_id) = owner_id_str.parse::<u64>().ok() {
                                if sm.user.id.get() == owner_id {
                                    if let Some(message_id) = active.get(&user_interact_id).cloned()
                                    {
                                        if let Some(stop_tx) =
                                            COLLECTOR_STOPS.lock().await.remove(&user_interact_id)
                                        {
                                            let _ = stop_tx.send(()); // ส่งสัญญาณหยุด
                                        }
                                        if let Err(err) = sm
                                            .channel_id
                                            .delete_message(&ctx.http, message_id)
                                            .await
                                        {
                                            tracing::error!("Failed to delete message: {:?}", err);
                                        }

                                        tracing::debug!(
                                            "User {} has canceled the ban and removed from active bans",
                                            user_interact_id
                                        );
                                    }
                                }
                            }
                        }
                    }
                    _ => {
                        tracing::warn!(
                            "Unknown interaction type or custom_id: {}",
                            sm.data.custom_id
                        );
                    }
                }
            } else {
                let follow_up = CreateInteractionResponseFollowup::new()
                    .content("You are not owner of this command!")
                    .ephemeral(true);

                let response = sm.create_followup(&ctx.http, follow_up).await;

                if let Err(e) = response {
                    tracing::error!("Active Session Failed to send response: {:?}", e);
                }
            }
        }
    }
}

async fn handle_select(user_interact_id: UserId, values: Vec<UserId>) {
    {
        let mut bans = BANS.lock().await;

        if let Some(entry) = bans.iter_mut().find(|b| b.admin_id == user_interact_id) {
            // ถ้ามี admin เดิม → อัปเดต users ให้เท่ากับค่าที่เลือกปัจจุบัน
            entry.users = values;
        } else {
            // ถ้าไม่มี admin เดิม → สร้างใหม่
            bans.push(BanUserTarget {
                admin_id: user_interact_id,
                users: values,
            });
        }
    }
}
