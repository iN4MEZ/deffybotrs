use std::{env, sync::Arc, time::Duration};

use anyhow::Error;
use chrono::{DateTime, Utc};
use deffy_bot_patreon_services::{LastChrgeStatus, PatreonApi, PatronStatus};
use mongodb::{
    Client, Collection, Database,
    bson::{doc, to_bson},
};
use serde::{Deserialize, Serialize};
use tokio::{
    sync::{OnceCell, mpsc},
    time::sleep,
};

use mongodb::bson::DateTime as BsonDateTime;

static DB: OnceCell<Arc<Database>> = OnceCell::const_new();
static TX_EVENT: OnceCell<mpsc::UnboundedSender<ScheduleMessage>> = OnceCell::const_new();

pub enum DatabaseWebHookEvent {
    CreateMember,
    UpdateMember,
    DeleteMember,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PatreonUserData {
    pub patreon_email: Option<String>,
    pub patreon_username: String,
    pub patreon_status: Option<PatronStatus>,
    pub last_charge_date: Option<BsonDateTime>,
    pub last_charge_status: Option<LastChrgeStatus>,
    pub next_charge_date: Option<BsonDateTime>,
    pub lifetime_support_cents: i64,
}

#[derive(Debug)]
pub enum ScheduleMessage {
    Info(String),
    Error(String),
    ForceUpdate,
}

pub struct DatabaseManager {}

impl DatabaseManager {
    pub async fn init_db() -> Result<Self, Error> {
        let mongo_uri = env::var("MONGO_URI").expect("MONGO_URI must be set");

        let mongo_client = Arc::new(Client::with_uri_str(mongo_uri).await?);

        let db: Database = mongo_client.database("patreon_api_data");

        DB.set(Arc::new(db)).expect("Already initialized");

        Ok(Self {})
    }

    pub fn get_db() -> Arc<Database> {
        DB.get().expect("DB not initialized").clone()
    }

    pub fn force_update_patreon_data() {
        if let Some(tx) = TX_EVENT.get() {
            let _ = tx.send(ScheduleMessage::ForceUpdate);
        }
    }

    pub async fn start_collect(&self) -> Result<(), Error> {
        let db = Self::get_db();

        let (tx_log, mut rx_log) = mpsc::unbounded_channel::<ScheduleMessage>();

        let (tx_event, rx_event) = mpsc::unbounded_channel::<ScheduleMessage>();

        TX_EVENT.set(tx_event).expect("TX_EVENT already set");

        Self::collect_patreon_api_data_db_loop(db, rx_event, tx_log).await?;

        tokio::spawn(async move {
            while let Some(msg) = rx_log.recv().await {
                match msg {
                    ScheduleMessage::Info(info) => tracing::trace!("{}", info),
                    ScheduleMessage::Error(err) => tracing::error!("{}", err),

                    _ => {}
                }
            }
        });

        Self::force_update_patreon_data();

        Ok(())
    }

    async fn collect_patreon_api_data_db_loop(
        db: Arc<Database>,
        mut rx: mpsc::UnboundedReceiver<ScheduleMessage>,
        tx: mpsc::UnboundedSender<ScheduleMessage>,
    ) -> Result<(), Error> {
        tokio::spawn(async move {
            let collection: Collection<PatreonUserData> = db.collection("user_data");

            let api = PatreonApi {
                access_token: env::var("PATREON_ACCESS_TOKEN")
                    .expect("PATREON_ACCESS_TOKEN must be set"),
                ..Default::default()
            };

            loop {
                let interval = sleep(Duration::from_secs(86400));
                tokio::pin!(interval);

                tokio::select! {

                    _ = &mut interval => {
                    if let Err(e) = Self::fetch_update_patreon_data(&api, &collection, &tx).await {
                        let _ = tx.send(ScheduleMessage::Error(format!(
                            "Failed to fetch and update Patreon data: {}", e
                        )));
                    } else {
                        let _ = tx.send(ScheduleMessage::Info("24 hours passed, updating Patreon data".to_string()));
                    }
                }

                 Some(msg) = rx.recv() => {
                    match msg {
                        ScheduleMessage::ForceUpdate => {
                            if let Err(e) = Self::fetch_update_patreon_data(&api, &collection, &tx).await {
                                let _ = tx.send(ScheduleMessage::Error(format!(
                                    "Failed to fetch and update Patreon data (force): {}", e
                                )));
                            } else {
                                let _ = tx.send(ScheduleMessage::Info("Force update Patreon data".to_string()));
                                continue;
                            }
                        }
                        other => {
                            let _ = tx.send(ScheduleMessage::Info(format!("Received: {:?}", other)));
                        }

                    }

                 }

                }
            }
        });
        Ok(())
    }

    async fn fetch_update_patreon_data(
        api: &PatreonApi,
        collection: &Collection<PatreonUserData>,
        tx: &mpsc::UnboundedSender<ScheduleMessage>,
    ) -> Result<(), Error> {
        let api_rsp = api.all_members().await;
        match api_rsp {
            Ok(api_rsp) => {
                let api_vec: Vec<PatreonUserData> = api_rsp
                    .iter()
                    .filter_map(|member| {
                        let attr = &member.attributes;

                        Some(PatreonUserData {
                            patreon_email: attr.email.clone(),
                            patreon_username: attr.full_name.clone(),
                            patreon_status: attr.patron_status.clone(),
                            last_charge_date: attr.last_charge_date.map(convert_to_bson_datetime),
                            next_charge_date: attr.next_charge_date.map(convert_to_bson_datetime),
                            last_charge_status: attr.last_charge_status,
                            lifetime_support_cents: attr.lifetime_support_cents,
                        })
                    })
                    .collect();

                let _ = tx.send(ScheduleMessage::Info(format!(
                    "Updated {} items",
                    api_vec.len()
                )));

                // Clear the collection before inserting new data
                if let Err(e) = collection.delete_many(doc! {}).await {
                    let _ = tx.send(ScheduleMessage::Error(format!("DB clear error: {e}")));
                }

                if let Err(e) = collection.insert_many(api_vec).await {
                    let _ = tx.send(ScheduleMessage::Error(format!(" DB insert error: {e}")));
                }
            }
            Err(e) => {
                let _ = tx.send(ScheduleMessage::Error(format!(" DB error: {e}")));
            }
        }
        Ok(())
    }

    pub async fn update_discord_server_data<T>(sv_id: u64, key: &str, value: T) -> Result<(), Error>
    where
        T: Serialize,
    {
        let db = DatabaseManager::get_db();
        let collection: Collection<DiscordServerData> = db.collection("server_data");

        let filter = doc! { "server_id": sv_id as i64 };

        let bson_value = to_bson(&value).map_err(|e| Error::from(e))?;

        let update = doc! {
            "$set": {
                key: bson_value,
            }
        };

        if let Err(e) = collection.update_one(filter, update).upsert(true).await {
            return Err(e.into());
        }
        Ok(())
    }

    pub async fn get_discord_server_data(sv_id: u64) -> Result<DiscordServerData, Error> {
        let db = DatabaseManager::get_db();
        let collection: Collection<DiscordServerData> = db.collection("server_data");

        let filter = doc! { "server_id": sv_id as i64 };

        let data = collection.find_one(filter).await?;

        Ok(data.unwrap())
    }
}

pub struct PatreonVerification {
    pub patreon_email: String,
}

impl PatreonVerification {
    pub fn new(patreon_email: String) -> Self {
        Self { patreon_email }
    }

    pub async fn verify(&self) -> Result<(bool, &String), Error> {
        let db = DatabaseManager::get_db();

        let collection: Collection<PatreonUserData> = db.collection("user_data");

        let verify = collection
            .find_one(doc! { "patreon_email": self.patreon_email.clone() })
            .await?;

        let is_active = verify.as_ref().and_then(|v| v.patreon_status);

        if let Some(PatronStatus::ActivePatron) = is_active {
            return Err(anyhow::Error::msg("409"));
        }

        Ok((
            verify.as_ref().is_some()
                && verify.as_ref().unwrap().patreon_status.is_some()
                && !(verify.as_ref().unwrap().patreon_status.unwrap()
                    == PatronStatus::ActivePatron),
            &self.patreon_email,
        ))
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DiscordServerData {
    pub verify_role_id: u64,
    pub log_channel_id: u64,
    pub webhook_create_member_channel_id: u64,
    pub webhook_update_member_channel_id: u64,
    pub webhook_delete_member_channel_id: u64,
}

pub struct DiscordServerDatabaseManager {}

impl DiscordServerDatabaseManager {
    pub async fn get_verify_roles() -> Option<u64> {
        let guild_id = env::var("GUILD_ID").expect("GUILD_ID must be set");

        let data = DatabaseManager::get_discord_server_data(guild_id.parse::<u64>().unwrap()).await;

        if let Ok(data) = data {
            return Some(data.verify_role_id);
        }
        None
    }

    pub async fn get_logging_channel() -> Option<u64> {
        let guild_id = env::var("GUILD_ID").expect("GUILD_ID must be set");

        let data = DatabaseManager::get_discord_server_data(guild_id.parse::<u64>().unwrap()).await;

        if let Ok(data) = data {
            return Some(data.log_channel_id);
        }
        None
    }

    pub async fn get_webhook_patreon_channel() -> Option<DiscordServerData> {
        let guild_id = env::var("GUILD_ID").expect("GUILD_ID must be set");

        let data = DatabaseManager::get_discord_server_data(guild_id.parse::<u64>().unwrap()).await;

        if let Ok(data) = data {
            return Some(data);
        }
        None
    }

    pub async fn set_verify_roles(sv_id: u64, id: u64) -> Result<(), Error> {
        DatabaseManager::update_discord_server_data(sv_id, "verify_role_id", id).await?;
        Ok(())
    }

    pub async fn set_logging_channel(sv_id: u64, channel_id: u64) -> Result<(), Error> {
        DatabaseManager::update_discord_server_data(sv_id, "log_channel_id", channel_id).await?;
        Ok(())
    }

    pub async fn set_webhook_channel(
        sv_id: u64,
        channel_id: u64,
        event: DatabaseWebHookEvent,
    ) -> Result<(), Error> {
        match event {
            DatabaseWebHookEvent::CreateMember => {
                DatabaseManager::update_discord_server_data(
                    sv_id,
                    "webhook_create_member_channel_id",
                    channel_id,
                )
                .await?;
            }
            DatabaseWebHookEvent::UpdateMember => {
                DatabaseManager::update_discord_server_data(
                    sv_id,
                    "webhook_update_member_channel_id",
                    channel_id,
                )
                .await?;
            }
            DatabaseWebHookEvent::DeleteMember => {
                DatabaseManager::update_discord_server_data(
                    sv_id,
                    "webhook_delete_member_channel_id",
                    channel_id,
                )
                .await?;
            }
        }
        Ok(())
    }
}

fn convert_to_bson_datetime(chrono_dt: DateTime<Utc>) -> BsonDateTime {
    BsonDateTime::from_millis(chrono_dt.timestamp_millis())
}
