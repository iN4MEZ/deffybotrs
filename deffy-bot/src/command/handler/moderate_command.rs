use std::{sync::Arc, time::Duration};

use anyhow::Error;
use chrono::Utc;
use deffy_bot_macro::{command, event};
use once_cell::sync::Lazy;
use serenity::{
    all::{
        ButtonStyle, Colour, CommandInteraction, CommandOptionType, ComponentInteraction,
        ComponentInteractionCollector, ComponentInteractionDataKind, Context, CreateActionRow,
        CreateButton, CreateCommand, CreateCommandOption, CreateEmbed, CreateEmbedFooter,
        CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage,
        CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption, EditInteractionResponse,
        Permissions, UserId,
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
struct BanUserTarget {
    admin_id: Option<UserId>,
    users: Vec<UserId>,
    // reson: Option<String>,
}

static BANS: Lazy<Arc<Mutex<Vec<BanUserTarget>>>> = Lazy::new(|| Arc::new(Mutex::new(Vec::new())));

struct CollectorTxKey;

impl TypeMapKey for CollectorTxKey {
    type Value = mpsc::Sender<()>;
}

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

        //interaction.defer_ephemeral(&ctx.http).await?;

        match subcommand {
            "ban" => {
                send_ban_menu(&ctx, &interaction).await?;
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

async fn send_ban_menu(ctx: &Context, interaction: &CommandInteraction) -> Result<(), Error> {
    let run_owner_id = &interaction.user.id;

    let custom_id = format!("banuser:{}", &run_owner_id);

    let confirm_btn_owner_custom_id = format!("confirmbanbtn:{}", &run_owner_id);

    EVENT_ROUTER.register("moderate:event", &run_owner_id);

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
    );

    let components = vec![select_menu, ban_rule_id_select_menu, action_row_btn];

    let timeout = Duration::from_secs(30);

    let message_embed = CreateEmbed::new()
        .color(0xffa800)
        .title("MODERATOR BAN MENU")
        .description("Select *user's* to ban from the server.")
        .footer(CreateEmbedFooter::new(format!("Run by: {}", interaction.user.name)))
        .thumbnail("https://static.wikia.nocookie.net/zenless-zone-zero/images/c/ce/Base_Bangboo_Portrait.png")
        .timestamp(Utc::now());

    interaction
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .embeds(vec![message_embed])
                    .components(components)
                    .ephemeral(true),
            ),
        )
        .await?;

    let (tx, rx) = mpsc::channel::<()>(1);

    {
        let mut data = ctx.data.write().await;
        data.insert::<CollectorTxKey>(tx.clone());
    }

    tokio::spawn({
        let ctx = ctx.clone();
        async move {
            if let Err(e) = run_collector(
                &ctx,
                custom_id,
                confirm_btn_owner_custom_id,
                timeout,
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

pub async fn run_collector(
    ctx: &Context,
    custom_id: String,
    confirm_btn_owner_custom_id: String,
    timeout_duration: Duration,
    mut stop_rx: mpsc::Receiver<()>,
) -> serenity::Result<()> {
    let mut collector = ComponentInteractionCollector::new(&ctx.shard)
        .filter({
            let custom_id = custom_id.clone();
            move |mci| mci.data.custom_id == custom_id
        })
        .timeout(timeout_duration)
        .stream();

    let mut last_interaction: Option<ComponentInteraction> = None;

    loop {
        tracing::debug!("run_collector: waiting for interaction or stop signal");
        tokio::select! {
            // stop signal มาก่อน → ออก
            biased;

            _ = stop_rx.recv() => {
                tracing::info!("stop signal received");
                break;
            }

            maybe_interaction = collector.next() => {
                match maybe_interaction {
                    Some(interaction) => {

                        last_interaction = Some(interaction.clone()); // clone หรือ Arc

                        if let ComponentInteractionDataKind::UserSelect { values } = &interaction.data.kind {
                            let user_count = values.len();
                            tracing::info!("Selected users count: {}", user_count);

                            let select_menu = create_select_menu(
                                &custom_id,
                                "please select users",
                                CreateSelectMenuKind::User {
                                    default_users: Some(Vec::new()),
                                },
                            );

                            let action_row_btn = create_btn_ban_action_row(
                                !(user_count > 0),
                                &confirm_btn_owner_custom_id,
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
                        tracing::info!("collector stream ended (timeout/drop)");

                        let run_owner_id = custom_id.split(':').nth(1).unwrap().parse::<u64>().unwrap();

                        tracing::info!("Cleaning up collector for user: {}", run_owner_id);

                        EVENT_ROUTER.unregister("moderate:event", run_owner_id.into());
                        break;
                    }
                }
            }
        }
    }

    if let Some(interaction) = last_interaction {
        let embed = CreateEmbed::default()
            .title("BAN MENU ENDED")
            .description("Ban menu ended or time out.")
            .color(Colour::new(0x0062ff));

        interaction
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new()
                    .components(vec![])
                    .embed(embed),
            )
            .await?;
    }

    Ok(())
}

fn create_btn_ban_action_row(
    is_disabled: bool,
    custom_confirm_id: &str,
) -> CreateActionRow {
    let confirm_action_btn = CreateButton::new(custom_confirm_id)
        .style(ButtonStyle::Success)
        .label("CONFIRM BAN")
        .disabled(is_disabled);


    let reason_action_btn = CreateButton::new("banreason")
        .style(ButtonStyle::Secondary)
        .label("REASON")
        .disabled(is_disabled);

    let attactment_action_btn = CreateButton::new("banattachment")
        .style(ButtonStyle::Primary)
        .label("ATTACHMENT")
        .disabled(is_disabled);

    let ban_duration_action_btn = CreateButton::new("banduration")
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

#[event(e = interaction_create, route = "moderate:event")]
async fn on_interaction(ctx: Context, data: EventData) -> anyhow::Result<()> {
    if let EventData::Interaction(interaction) = data {
        
        if let Some(msci) = interaction.as_message_component() {
            let user_interact_id = msci.user.id;

            let select_menu_id = format!("banuser:{}", user_interact_id);
            let custom_id = msci.data.custom_id.as_str();

            match custom_id {
                id if id.starts_with("confirmbanbtn:") => {
                    if let Some(owner_id_str) = id.strip_prefix("confirmbanbtn:") {
                        if let Ok(owner_id) = owner_id_str.parse::<u64>() {
                            if msci.user.id.get() == owner_id {
                                let ban_result =
                                    handle_moderate_action(&ctx, msci, &ModeratorAction::Ban).await;

                                if let Err(err) = ban_result {
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
                        }
                    }
                }

                id if id == select_menu_id => {
                    if let ComponentInteractionDataKind::UserSelect { values } = &msci.data.kind {
                        handle_select(user_interact_id, values.clone()).await;
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
async fn handle_select(user_interact_id: UserId, values: Vec<UserId>) {
    {
        let mut bans = BANS.lock().await;

        if let Some(entry) = bans
            .iter_mut()
            .find(|b| b.admin_id == Some(user_interact_id))
        {
            // ถ้ามี admin เดิม → อัปเดต users ให้เท่ากับค่าที่เลือกปัจจุบัน
            entry.users = values;
        } else {
            // ถ้าไม่มี admin เดิม → สร้างใหม่
            bans.push(BanUserTarget {
                admin_id: Some(user_interact_id),
                users: values,
            });
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
            let bans = BANS.lock().await;

            for user in bans.iter() {
                if user.admin_id == Some(user_interact_id) {
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
                                    "Cannot ban {} with ADMINISTRATOR permission",target_user.to_user(&ctx.http).await?.name
                                ));
                            }
                        }

                        // test with dm
                        let dm = target_user.create_dm_channel(&ctx.http).await;

                        if dm.is_ok() {
                            let dm_channel = dm.unwrap();
                            let content = format!(
                                "You have been banned by <@{}>.\nReason: Gay",
                                user_interact_id
                            );

                            dm_channel
                                .send_message(&ctx.http, CreateMessage::new().content(content))
                                .await?;
                        } else {
                            tracing::warn!("Failed to create DM channel with user: {}", target_user);
                        }

                        let result = ctx
                            .http
                            .ban_user(
                                interaction.guild_id.unwrap(),
                                target_user.clone(),
                                0, // 0 means permanent ban
                                Some("Banned by moderator command"),
                            )
                            .await;

                        match result {
                            Ok(_) => {
                                if let Some(tx) = {
                                    let data = ctx.data.read().await;
                                    data.get::<CollectorTxKey>().cloned()
                                } {
                                    let _ = tx.send(()).await;
                                }
                            }
                            Err(err) => {
                                return Err(anyhow::anyhow!("Failed to ban user: {:?}", err));
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
