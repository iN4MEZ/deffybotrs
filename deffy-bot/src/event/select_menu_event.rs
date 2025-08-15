use std::sync::Arc;

use deffy_bot_macro::event;
use once_cell::sync::Lazy;
use serenity::{
    all::{
        ComponentInteractionDataKind, Context, CreateInteractionResponse,
        CreateInteractionResponseMessage, CreateMessage,
    },
    futures::future::join_all,
    model::id,
};
use tokio::sync::Mutex;

use crate::{command::moderate_command::{ACTIVE_BANS_MENU, COLLECTOR_STOPS}, event::manager::EventData};

static BANS: Lazy<Arc<Mutex<Vec<BanUserTarget>>>> = Lazy::new(|| Arc::new(Mutex::new(Vec::new())));

#[derive(Debug, Clone)]
struct BanUserTarget {
    admin_id: id::UserId,
    users: Vec<id::UserId>,
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
                            if let Ok(owner_id) = owner_id_str.parse::<u64>() {
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
                                            if let Ok(dm_channel) = dm {
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
                                        tracing::error!("Ban Confirm: Failed to send response: {:?}", e);
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
                                .create_response(
                                    &ctx.http,
                                    CreateInteractionResponse::Acknowledge
                                )
                                .await;
                            if let Err(e) = response {
                                tracing::error!("Select Menu: Failed to send response: {:?}", e);
                            }
                        }
                    }

                    id if id.starts_with("cancelbanbtn:") => {
                        if let Some(owner_id_str) = id.strip_prefix("cancelbanbtn:") {
                            if let Ok(owner_id) = owner_id_str.parse::<u64>() {
                                if sm.user.id.get() == owner_id {
                                    if let Some(message_id) = active.get(&user_interact_id).cloned()
                                    {
                                        if let Some(stop_tx) = COLLECTOR_STOPS
                                            .lock()
                                            .await
                                            .remove(&user_interact_id)
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
                let response = sm
                    .create_response(
                        &ctx.http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .content("You have no active ban session.")
                                .ephemeral(true),
                        ),
                    )
                    .await;

                if let Err(e) = response {
                    tracing::error!("Active Session Failed to send response: {:?}", e);
                }
            }
        }
    }
}

async fn handle_select(user_interact_id: id::UserId, values: Vec<id::UserId>) {
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
