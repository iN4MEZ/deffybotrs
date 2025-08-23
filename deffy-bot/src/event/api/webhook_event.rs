use deffy_bot_macro::event_handle;
use deffy_bot_utils::database::DiscordServerDatabaseManager;
use deffy_bot_utils::event::manager::{EventInfo, EventTypeData};
use deffy_bot_utils::event::manager::EventType::PatreonWebhookUserCreated;
use deffy_bot_utils::event::manager::EventType::PatreonWebhookUserUpdated;
use deffy_bot_utils::event::manager::EventType::PatreonWebhookUserDeleted;
use serenity::all::{ChannelId, Colour, CreateEmbed, CreateEmbedFooter, CreateMessage, Timestamp};

use crate::event::start_event::BOT_HTTP;

#[event_handle(e = PatreonWebhookUserCreated)]
pub async fn handle_patreon_webhook_user_created(data: EventTypeData) -> Result<(), anyhow::Error> {
    let http = BOT_HTTP.get().expect("BOT_HTTP not initialized");

    match data {
        EventTypeData::PatreonMemberData(data) => {
            if let Some(discord_db) =
                DiscordServerDatabaseManager::get_webhook_patreon_channel().await
            {
                let channel_id = ChannelId::new(discord_db.webhook_create_member_channel_id);

                let embed = CreateEmbed::default()
                    .title("🆕 NEW MEMBER JOINED")
                    .description(format!(
                        "*New user has been join patreon!*\n*{}*\n*status:{:?}*",
                        data.attributes.full_name, data.attributes.patron_status
                    ))
                    .color(Colour::new(0x53d0b1))
                    .timestamp(Timestamp::now())
                    .thumbnail("https://static.wikia.nocookie.net/zenless-zone-zero/images/c/ce/Base_Bangboo_Portrait.png")
                    .footer(CreateEmbedFooter::new("Join date"));

                let builder = CreateMessage::new().embed(embed);

                channel_id.send_message(&http, builder).await?;
            }
        }
    }

    Ok(())
}

#[event_handle(e = PatreonWebhookUserUpdated)]
pub async fn handle_patreon_webhook_user_updated(data: EventTypeData) -> Result<(), anyhow::Error> {
    let http = BOT_HTTP.get().expect("BOT_HTTP not initialized");

    match data {
        EventTypeData::PatreonMemberData(data) => {
            if let Some(discord_db) =
                DiscordServerDatabaseManager::get_webhook_patreon_channel().await
            {
                let channel_id = ChannelId::new(discord_db.webhook_update_member_channel_id);

                let embed = CreateEmbed::default()
                    .title("⚙️ MEMBER HAS UPDATED")
                    .description(format!(
                        "*The user has been updated!*\n*{}*\n*status:{:?}*\n*last_charge_status:{:?}*\n*last_charge_date:{:?}*\n*next_charge_date:{:?}*\n",
                        data.attributes.full_name, data.attributes.patron_status,data.attributes.last_charge_status,data.attributes.last_charge_date,data.attributes.next_charge_date
                    ))
                    .color(Colour::new(0xf5d400))
                    .timestamp(Timestamp::now())
                    .thumbnail("https://static.wikia.nocookie.net/zenless-zone-zero/images/5/5c/Cryboo_Portrait.png")
                    .footer(CreateEmbedFooter::new("Update time"));

                let builder = CreateMessage::new().embed(embed);

                channel_id.send_message(&http, builder).await?;
            }
        }
    }

    Ok(())
}

#[event_handle(e = PatreonWebhookUserDeleted)]
pub async fn handle_patreon_webhook_user_deleted(data: EventTypeData) -> Result<(), anyhow::Error> {
    let http = BOT_HTTP.get().expect("BOT_HTTP not initialized");

    match data {
        EventTypeData::PatreonMemberData(data) => {
            if let Some(discord_db) =
                DiscordServerDatabaseManager::get_webhook_patreon_channel().await
            {
                let channel_id = ChannelId::new(discord_db.webhook_delete_member_channel_id);

                let embed = CreateEmbed::default()
                    .title("❌ MEMBER HAS CANCELED")
                    .description(format!(
                        "*The user has been canceled**\n*{}*\n*status:{:?}*\n*last_charge_status:{:?}*\n*last_charge_date:{:?}*\n",
                        data.attributes.full_name, data.attributes.patron_status,data.attributes.last_charge_status,data.attributes.last_charge_date
                    ))
                    .color(Colour::new(0xff0026))
                    .timestamp(Timestamp::now())
                    .thumbnail("https://static.wikia.nocookie.net/zenless-zone-zero/images/6/64/Avocaboo_Portrait.png")
                    .footer(CreateEmbedFooter::new("Cancel time"));

                let builder = CreateMessage::new().embed(embed);

                channel_id.send_message(&http, builder).await?;
            }
        }
    }

    Ok(())
}
