use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::Error;
use chrono::Utc;
use deffy_bot_macro::{command, event};
use deffy_bot_utils::builder_utils::ModalBuilder;
use serenity::{
    all::{
        ButtonStyle, Colour, CommandInteraction, CommandOptionType, ComponentInteraction, ComponentInteractionCollector, ComponentInteractionDataKind, Context, CreateActionRow, CreateButton, CreateCommand, CreateCommandOption, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage, CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption, EditInteractionResponse, ModalInteraction, Permissions, UserId
    },
    async_trait,
    futures::StreamExt,
    prelude::TypeMapKey,
};
use tokio::{
    sync::{Mutex, mpsc},
    time::timeout,
};

use crate::{
    command::system::manager::{CommandHandler, CommandInfo},
    event::{event_router::EVENT_ROUTER, manager::EventData},
};

enum ModeratorAction {
    Ban,
}

#[derive(Debug, Clone)]
pub struct BanData {
    users: Vec<UserId>,
    ban_info: BanInfo,
    collector_tx: Option<mpsc::Sender<CollectorSignal>>,
}

#[derive(Debug, Clone)]
struct BanInfo {
    reason: Option<String>,
    duration: Option<u8>,
    attachment: Option<String>,
}

type UpdateFn =
    fn(&mut BanData, String) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

enum CollectorSignal {
    Stop
}

pub struct BanSession;

impl TypeMapKey for BanSession {
    type Value = Arc<Mutex<HashMap<UserId, BanData>>>;
}

enum BanMenuStatus {
    RUNNING,
    ENDED,
}

const TIMEOUT: i32 = 60;

#[command(cmd = moderator, cooldown = 5)]
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

        match subcommand {
            "ban" => {
                handle_ban_command(&ctx, &interaction).await?;
            }
            _ => {}
        }

        Ok(())
    }

    fn register(&self) -> CreateCommand {
        CreateCommand::new(self.name())
            .description("A moderator command for admin")
            .default_member_permissions(Permissions::BAN_MEMBERS | Permissions::KICK_MEMBERS)
            .add_option(CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "ban",
                "ban user",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "warn",
                "warn user",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "kick",
                "kick user",
            ))
    }
}

async fn handle_ban_command(ctx: &Context, interaction: &CommandInteraction) -> Result<(), Error> {
    start_collector(ctx, interaction).await?;
    Ok(())
}
async fn start_collector(ctx: &Context, interaction: &CommandInteraction) -> Result<(), Error> {
    let user_id = &interaction.user.id;

    let rx = create_user_session(user_id, ctx).await;

    let timeout = Duration::from_secs(TIMEOUT as u64);
    
    let ctx_clone = ctx.clone();
    let user_id_clone = user_id.clone();
    let interaction_clone = interaction.clone();


    tokio::spawn({
        async move {
            if let Err(e) = run_collector(
                ctx_clone,
                user_id_clone,
                timeout,
                interaction_clone,
                rx,
            )
            .await
            {
                tracing::error!("Error in run_collector: {:?}", e);
            }
        }
    });

    Ok(())
}

async fn run_collector(
    ctx: Context,
    user_session_id: UserId,
    timeout_duration: Duration,
    interaction: CommandInteraction,
    mut stop_rx: mpsc::Receiver<CollectorSignal>,
) -> serenity::Result<()> {
    let sm_custom_id = format!("banuser:{}", &user_session_id);

    let confirm_btn_owner_custom_id = format!("confirmbanbtn:{}", &user_session_id);

    let reason_btn_owner_custom_id = format!("reasonbanbtn:{}", &user_session_id);

    let duration_btn_owner_custom_id = format!("durationbanbtn:{}", &user_session_id);

    let attachment_btn_owner_custom_id = format!("attachmentbanbtn:{}", &user_session_id);

    EVENT_ROUTER.register("moderate:event", &user_session_id);

    let select_menu = create_select_menu(
        &sm_custom_id,
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
        &reason_btn_owner_custom_id,
        &duration_btn_owner_custom_id,
        &attachment_btn_owner_custom_id,
    );

    let components = vec![select_menu, ban_rule_id_select_menu, action_row_btn];

    let message_embed = ban_menu_embed(
        "No reason provided".to_string(),
        "0".to_string(),
        "No attachment".to_string(),
        interaction.user.name.clone(),
        BanMenuStatus::RUNNING,
    )
    .await;

    interaction
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .embed(message_embed)
                    .components(components)
                    .ephemeral(true),
            ),
        )
        .await?;


    let last_interaction: Option<CommandInteraction> = Some(interaction.clone());

    let mut collector = ComponentInteractionCollector::new(&ctx.shard)
        .filter({
            let custom_id = sm_custom_id.clone();
            move |mci| mci.data.custom_id == custom_id
        })
        .timeout(timeout_duration)
        .author_id(user_session_id)
        .stream();

    loop {
        tracing::debug!("run_collector: waiting for interaction or stop signal");
        tokio::select! {
            biased; // Ensure this is inside the select! block

            maybe_signal = stop_rx.recv() => {
                if let Some(signal) = maybe_signal {
                    match signal {
                        CollectorSignal::Stop => {
                            tracing::debug!("run_collector: stop signal received");
                            break;
                        }
                    }
                }
            }

            maybe_interaction = collector.next() => {
                match maybe_interaction {
                    Some(interaction) => {
                        if let ComponentInteractionDataKind::UserSelect { values } = &interaction.data.kind {
                            
                            let user_count = values.len();

                            tracing::info!("Selected users count: {}", user_count);

                            let select_menu = create_select_menu(
                                &sm_custom_id,
                                "please select users",
                                CreateSelectMenuKind::User {
                                    default_users: Some(Vec::new()),
                                },
                            );

                            let action_row_btn = create_btn_ban_action_row(
                                !(user_count > 0),
                                &confirm_btn_owner_custom_id,
                                &reason_btn_owner_custom_id,
                                &duration_btn_owner_custom_id,
                                &attachment_btn_owner_custom_id,
                            );

                            let components = vec![select_menu, action_row_btn];

                            // กันไว้เผื่อ edit_response ค้าง
                            match timeout(Duration::from_secs(10),
                                          interaction.edit_response(
                                              &ctx.http,
                                              EditInteractionResponse::new().components(components),
                                          )
                            ).await {
                                Ok(Ok(_)) => {}
                                Ok(Err(e)) => tracing::error!("edit_response error: {:?}", e),
                                Err(_) => tracing::warn!("edit_response timed out"),
                            }
                        }
                    }
                    None => {
                        tracing::debug!("collector stream ended (timeout/drop)");
                        break;
                    }
                }
            }
        }
    }

    EVENT_ROUTER.unregister("moderate:event", user_session_id.into());

    if let Some(interaction) = last_interaction {
        let message_embed = ban_menu_embed(
            "No reason provided".to_string(),
            "0".to_string(),
            "No attachment".to_string(),
            interaction.user.name.clone(),
            BanMenuStatus::ENDED,
        )
        .await;

        // disable ปุ่มทั้งหมด
        let action_row_btn = create_btn_ban_action_row(
            true,
            &confirm_btn_owner_custom_id,
            &reason_btn_owner_custom_id,
            &duration_btn_owner_custom_id,
            &attachment_btn_owner_custom_id,
        );

        interaction
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new()
                    .embed(message_embed)
                    .components(vec![action_row_btn]),
            )
            .await?;
    }

    Ok(())
}

async fn create_user_session(uid: &UserId,ctx: &Context) -> mpsc::Receiver<CollectorSignal> {
    let data_read = ctx.data.read().await;
    let map = data_read.get::<BanSession>().unwrap().clone();
    drop(data_read);

    {
        if let Some(old_tx) = map.lock().await.remove(&uid) {
            if let Some(tx) = old_tx.collector_tx {
                let _ = tx.send(CollectorSignal::Stop).await;
            }
        }
    }

    let (tx, rx) = mpsc::channel::<CollectorSignal>(1);

    {
        map.lock().await.insert(uid.into(), BanData {
            users: vec![],
            ban_info: BanInfo {
                reason: None,
                duration: None,
                attachment: None,
            },
            collector_tx: Some(tx.clone()),

        });
    }
    rx

}

#[event(e = interaction_create, route = "moderate:event")]
async fn on_interaction(ctx: Context, data: EventData) -> anyhow::Result<()> {
    if let EventData::Interaction(interaction) = data {
        if let Some(msci) = interaction.as_message_component() {
            let user_interact_id = msci.user.id;

            let select_menu_id = format!("banuser:{}", user_interact_id);
            let custom_id = msci.data.custom_id.as_str();

            match custom_id {
                id if id.starts_with("reasonbanbtn:") => {
                    msci.create_response(&ctx.http, build_modal("reason").build())
                        .await?;
                }

                id if id.starts_with("durationbanbtn:") => {
                    msci.create_response(&ctx.http, build_modal("duration").build())
                        .await?;
                }

                id if id.starts_with("attachmentbanbtn:") => {
                    msci.create_response(&ctx.http, build_modal("attachment").build())
                        .await?;
                }

                id if id.starts_with("confirmbanbtn:") => {
                    if let Err(err) =
                        handle_moderate_action(&ctx, msci, &ModeratorAction::Ban).await
                    {
                        msci.create_response(
                            &ctx.http,
                            CreateInteractionResponse::Message(
                                CreateInteractionResponseMessage::new()
                                    .content(format!("```Error: {}```", err))
                                    .ephemeral(true),
                            ),
                        )
                        .await?;
                    }
                }

                id if id == select_menu_id => {
                    if let ComponentInteractionDataKind::UserSelect { values } = &msci.data.kind {
                        handle_select(&ctx,user_interact_id, values.clone()).await;
                        msci.create_response(&ctx.http, CreateInteractionResponse::Acknowledge)
                            .await?;
                    }
                }

                _ => {
                    tracing::warn!("Unknown interaction type or custom_id: {}", custom_id);
                }
            }
        }
    }
    Ok(())
}

#[event(e = interaction_create, route = "moderate:event")]
async fn interaction_event(ctx: Context, data: EventData) -> Result<(), anyhow::Error> {
    if let EventData::Interaction(interaction) = data {
        if let Some(modal) = &interaction.modal_submit() {
            if let Some((field_key, updater)) = get_modal_map().get(modal.data.custom_id.as_str()) {
                handle_modal_input(modal, &ctx, field_key, *updater).await?;
            } else {
                tracing::warn!("Unknown modal custom_id: {}", modal.data.custom_id);
            }

            // update ban menu embed

            let ban_info = get_ban_info(&ctx, &modal.user.id).await;

            let reason = ban_info.reason.unwrap();
            let duration = ban_info.duration.unwrap().to_string();
            let attachment = ban_info.attachment.unwrap();

            let message_embed = ban_menu_embed(
                reason,
                duration,
                attachment,
                modal.user.name.clone(),
                BanMenuStatus::RUNNING,
            )
            .await;

            modal
                .edit_response(
                    &ctx.http,
                    EditInteractionResponse::new().embed(message_embed),
                )
                .await?;
        }
    }
    Ok(())
}

async fn handle_select(ctx: &Context, user_interact_id: UserId, values: Vec<UserId>) {
    {
        let data_read = ctx.data.read().await;
        let bans = data_read.get::<BanSession>().unwrap().clone();

        let mut lock = bans.lock().await;

        if let Some(entry) = lock.get_mut(&user_interact_id) {
            // ถ้ามี admin เดิม → อัปเดต users ให้เท่ากับค่าที่เลือกปัจจุบัน
            entry.users = values;
            return;
        } else {
            // ถ้าไม่มี admin เดิม → สร้างใหม่
            lock.insert(
                user_interact_id,
                BanData {
                    users: values,
                    ban_info: {
                        BanInfo {
                            reason: None,
                            duration: None,
                            attachment: None,
                        }
                    },
                    collector_tx: None,
                },
            );
        }
    }
}

async fn handle_moderate_action(
    ctx: &Context,
    interaction: &ComponentInteraction,
    action: &ModeratorAction,
) -> Result<(), Error> {
    let user_interact_id = interaction.user.id;

    match action {
        ModeratorAction::Ban => {
            let admin = {
                let data_read = ctx.data.read().await;
                let bans = data_read.get::<BanSession>().unwrap().clone();

                let guard = bans.lock().await;

                guard.get(&user_interact_id).cloned()
            };

            for user in admin.iter() {
                for target_user in &user.users {
                    if target_user == &user_interact_id {
                        return Err(anyhow::anyhow!("Cannot ban yourself"));
                    }

                    let member_permissions = interaction
                        .guild_id
                        .and_then(|guild_id| guild_id.to_guild_cached(&ctx.cache))
                        .and_then(|guild| {
                            guild
                                .members
                                .get(target_user)
                                .map(|member| guild.member_permissions(member))
                        });

                    if let Some(permissions) = member_permissions {
                        if permissions.contains(Permissions::ADMINISTRATOR) {
                            return Err(anyhow::anyhow!(
                                "Cannot ban {} with ADMINISTRATOR permission",
                                target_user.to_user(&ctx.http).await?.name
                            ));
                        }
                    }

                    let ban_info = get_ban_info(&ctx,&user_interact_id).await;

                    let reason = ban_info.reason.as_deref();

                    tracing::info!(
                        "Banning user: {} by admin: {} for reason: {:?}",
                        target_user,
                        user_interact_id,
                        &reason
                    );

                    // test with dm
                    let dm = target_user.create_dm_channel(&ctx.http).await;

                    if dm.is_ok() {
                        let dm_channel = dm.unwrap();
                        let content = format!(
                            "You have been banned by <@{}>.\nReason: {:?}\nDuration: {} days\nAttachment: {}\nIf you believe this is a mistake, please contact the server administrators.",
                            user_interact_id,
                            &reason,
                            if ban_info.duration.unwrap() == 0 {
                                "Permanent".to_string()
                            } else {
                                ban_info.duration.unwrap().to_string()
                            },
                            ban_info.attachment.unwrap()
                        );

                        dm_channel
                            .send_message(&ctx.http, CreateMessage::new().content(content))
                            .await?;
                    } else {
                        tracing::warn!("Failed to create DM channel with user: {}", target_user);
                    }

                    // let result = ctx
                    //     .http
                    //     .ban_user(
                    //         interaction.guild_id.unwrap(),
                    //         target_user.clone(),
                    //         ban_info.duration.unwrap(), // 0 means permanent ban
                    //         ban_info.reason.as_deref(),
                    //     )
                    //     .await;

                    {
                        let data_read = ctx.data.read().await;
                        let map = data_read.get::<BanSession>().unwrap().clone();
                        drop(data_read);

                        if let Some(old_tx) = map.lock().await.remove(&user_interact_id) {
                            if let Some(tx) = old_tx.collector_tx {
                                let _ = tx.send(CollectorSignal::Stop).await;
                            }
                        }
                    }

                    // match result {
                    //     Ok(_) => {
                    //         if let Some(tx) = {
                    //             let data = ctx.data.read().await;
                    //             data.get::<CollectorTxKey>().cloned()
                    //         } {
                    //             let _ = tx.send(()).await;
                    //         }
                    //     }
                    //     Err(err) => {
                    //         return Err(anyhow::anyhow!("Failed to ban user: {:?}", err));
                    //     }
                    // }
                }
            }
        }
    }

    Ok(())
}

async fn ban_menu_embed(
    reason: String,
    duration: String,
    attachment: String,
    admin_name: String,
    state: BanMenuStatus,
) -> CreateEmbed {

    let use_field = {
        match state {
            BanMenuStatus::RUNNING => {
                vec![
                    ("Reason", reason, true),
                    ("Duration", format!("{} days", duration), true),
                    ("Attachment", attachment, true)
                ]
            },
            BanMenuStatus::ENDED => {
                vec![]
            }
        }
    };

    let message_embed = CreateEmbed::new()
        .color(match state {
            BanMenuStatus::RUNNING => Colour::ORANGE,
            BanMenuStatus::ENDED => Colour::RED,
        })
        .title(format!("Ban Menu - {}", match state {
            BanMenuStatus::RUNNING => "In Progress",
            BanMenuStatus::ENDED => "Ended",
        }))
        .description(match state {
            BanMenuStatus::RUNNING => "Select *user's* to ban from the server.",
            BanMenuStatus::ENDED => "The ban process has ended. please create a new ban menu to ban users.",
            
        })
        .footer(CreateEmbedFooter::new(format!("ADMIN: {}", admin_name)))
        .thumbnail("https://static.wikia.nocookie.net/zenless-zone-zero/images/c/ce/Base_Bangboo_Portrait.png")
        .timestamp(Utc::now())
        .fields(use_field);

    message_embed
}

fn build_modal(kind: &str) -> ModalBuilder {
    match kind {
        "reason" => ModalBuilder::new("BAN REASON", "Reason for ban").add_text_input(
            "ban_reason",
            "BAN REASON",
            serenity::all::InputTextStyle::Paragraph,
        ),

        "duration" => ModalBuilder::new("BAN DURATION", "Duration for ban").add_text_input(
            "ban_duration",
            "BAN DURATION (in days, 0 for permanent)",
            serenity::all::InputTextStyle::Short,
        ),

        "attachment" => ModalBuilder::new("BAN ATTACHMENT", "Attachment for ban").add_text_input(
            "ban_attachment",
            "BAN ATTACHMENT (URL)",
            serenity::all::InputTextStyle::Paragraph,
        ),

        _ => unreachable!(),
    }
}

fn get_modal_map() -> HashMap<&'static str, (&'static str, UpdateFn)> {
    let mut map: HashMap<&'static str, (&'static str, UpdateFn)> = HashMap::new();

    map.insert(
        "BAN REASON",
        ("ban_reason", |entry, v| {
            entry.ban_info.reason = Some(v);
            Ok(())
        }),
    );

    map.insert(
        "BAN DURATION",
        ("ban_duration", |entry, v| {
            let cast = v.parse::<u8>()?;
            entry.ban_info.duration = Some(cast);
            Ok(())
        }),
    );

    map.insert(
        "BAN ATTACHMENT",
        ("ban_attachment", |entry, v| {
            entry.ban_info.attachment = Some(v);
            Ok(())
        }),
    );

    map
}

async fn handle_modal_input(
    modal: &ModalInteraction,
    ctx: &Context,
    field_key: &str,
    update: UpdateFn,
) -> Result<(), serenity::Error> {
    let input_values = ModalBuilder::extract_modal_inputs(modal);

    let value = input_values
        .iter()
        .find(|(key, _)| key == field_key)
        .map(|(_, v)| v.clone())
        .unwrap_or_default();

    
    let data_read = ctx.data.read().await;
    let bans = data_read.get::<BanSession>().unwrap().clone();

    let mut bans_lock = bans.lock().await;

    if let Some(entry) = bans_lock.get_mut(&modal.user.id) {
        if let Err(err) = update(entry, value) {
            tracing::error!("Modal update error: {}", err);
        }
    }

    modal
        .create_response(&ctx.http, CreateInteractionResponse::Acknowledge)
        .await
}

async fn get_ban_info(ctx: &Context,user_id: &UserId) -> BanInfo {
    // lock ชั่วคราวเพื่อ clone ข้อมูล
    let current_ban = {

        let data_read = ctx.data.read().await;
        let bans = data_read.get::<BanSession>().unwrap().clone();

        let guard = bans.lock().await;
        guard.get(user_id).cloned()
    };

    // แปลงเป็นข้อมูลพร้อมใช้งาน
    let reason = current_ban
        .as_ref()
        .and_then(|b| b.ban_info.reason.clone())
        .unwrap_or_else(|| "No reason provided".to_string());

    let duration = current_ban
        .as_ref()
        .and_then(|b| {
            b.ban_info.duration.map(|d| {
                if d == 0 {
                    "Permanent".to_string()
                } else {
                    d.to_string()
                }
            })
        })
        .unwrap_or_else(|| "No duration set".to_string());

    let attachment = current_ban
        .as_ref()
        .and_then(|b| b.ban_info.attachment.clone())
        .unwrap_or_else(|| "No attachment".to_string());

    BanInfo {
        reason: Some(reason),
        duration: Some(duration.parse().unwrap_or(0)),
        attachment: Some(attachment),
    }
}

fn create_btn_ban_action_row(
    is_disabled: bool,
    custom_confirm_id: &str,
    custom_reason_id: &str,
    custom_duration_id: &str,
    custom_attachment_id: &str,
) -> CreateActionRow {
    let confirm_action_btn = CreateButton::new(custom_confirm_id)
        .style(ButtonStyle::Success)
        .label("CONFIRM BAN")
        .disabled(is_disabled);

    let reason_action_btn = CreateButton::new(custom_reason_id)
        .style(ButtonStyle::Secondary)
        .label("REASON")
        .disabled(is_disabled);

    let attactment_action_btn = CreateButton::new(custom_attachment_id)
        .style(ButtonStyle::Primary)
        .label("ATTACHMENT")
        .disabled(is_disabled);

    let ban_duration_action_btn = CreateButton::new(custom_duration_id)
        .style(ButtonStyle::Secondary)
        .label("DURATION")
        .disabled(is_disabled);


    let action_row_btn = CreateActionRow::Buttons(vec![
        confirm_action_btn,
        reason_action_btn,
        ban_duration_action_btn,
        attactment_action_btn,
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