use deffy_bot_macro::event_handle;
use deffy_bot_utils::event::manager::EventType::PatreonWebhookUserCreated;
use deffy_bot_utils::event::manager::{EventInfo, EventTypeData};
use serenity::all::{ChannelId, CreateMessage};

use crate::event::start_event::BOT_HTTP;

#[event_handle(e = PatreonWebhookUserCreated)]
pub async fn handle_bot_started(data: EventTypeData) -> Result<(), anyhow::Error> {
    let http = BOT_HTTP.get().expect("BOT_HTTP not initialized").clone();

    match data {
        EventTypeData::PatreonData(data) => {
            let channel_id = ChannelId::new(1276313312178733116);

            let builder = CreateMessage::new().content(format!("{} User has created!", data));

            channel_id.send_message(&http, builder).await?;
        }
    }

    Ok(())
}
