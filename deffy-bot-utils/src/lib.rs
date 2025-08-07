use std::{env, sync::Arc, time::Duration};

use anyhow::{Error};
use deffy_bot_patreon_services::PatreonApi;
use mongodb::{bson::{doc, to_bson}, Client, Collection, Database};
use serde::{Deserialize, Serialize};
use serenity::all::{
    ActionRowComponent, CreateActionRow, CreateInputText, CreateInteractionResponse, CreateModal,
    InputTextStyle, ModalInteraction,
};
use tokio::{sync::{mpsc, OnceCell}, time::sleep};


static DB: OnceCell<Arc<Database>> = OnceCell::const_new();

pub struct ModalBuilder {
    modal: CreateModal,
    components: Vec<CreateActionRow>,
}

impl ModalBuilder {
    pub fn new(custom_id: &str, title: &str) -> Self {
        Self {
            modal: CreateModal::new(custom_id, title),
            components: vec![],
        }
    }

    pub fn add_text_input(mut self, id: &str, label: &str, style: InputTextStyle) -> Self {
        let input = CreateInputText::new(style, label, id);
        let row = CreateActionRow::InputText(input);
        self.components.push(row);
        self
    }

    pub fn build(mut self) -> CreateInteractionResponse {
        self.modal = self.modal.components(self.components);
        CreateInteractionResponse::Modal(self.modal)
    }

    pub fn extract_modal_inputs(modal: &ModalInteraction) -> Vec<(String, String)> {
        modal
            .data
            .components
            .iter()
            .flat_map(|row| {
                row.components.iter().filter_map(|component| {
                    if let ActionRowComponent::InputText(input) = component {
                        Some((
                            input.custom_id.clone(),
                            input.value.clone().unwrap_or_default(),
                        ))
                    } else {
                        None
                    }
                })
            })
            .collect()
    }
}

#[derive(Serialize, Deserialize,Debug)]
pub struct PatreonUserData {
    pub patreon_email: String,
    pub patreon_username: String,
    pub patreon_status: String,
}

#[derive(Debug)]
pub enum ScheduleMessage {
    Info(String),
    Error(String),
}

pub struct DatabaseManager {
}

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

    pub async fn collect(&self) -> Result<(), Error> {

        let db = Self::get_db();

        let (tx, mut rx) = mpsc::unbounded_channel::<ScheduleMessage>();

        Self::collect_patreon_api_data_db(db, tx).await?;

        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                match msg {
                    ScheduleMessage::Info(info) => tracing::trace!("{}", info),
                    ScheduleMessage::Error(err) => tracing::error!("{}", err),
                }
            }
        });

        Ok(())
    }
    async fn collect_patreon_api_data_db(
        db: Arc<Database>,
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
                let api_rsp = api.all_members().await;
                match api_rsp {
                    Ok(api_rsp) => {
                        let api_vec: Vec<PatreonUserData> = api_rsp
                            .iter()
                            .filter_map(|member| {
                                let attr = &member.attributes;
    
                                if let (Some(email), full_name) = (&attr.email, &attr.full_name) {
                                    Some(PatreonUserData {
                                        patreon_email: email.clone(),
                                        patreon_username: full_name.clone(),
                                        patreon_status: attr.patron_status.clone().unwrap_or(deffy_bot_patreon_services::PatronStatus::Null).to_string(),
                                    })
                                } else {
                                    None
                                }
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
                sleep(Duration::from_secs(30*60)).await;
            }
        });
        Ok(())
    }

    pub async fn update_discord_server_data<T>(sv_id: u64,key: &str,value: T) -> Result<(), Error> where T: Serialize, {

        let db = DatabaseManager::get_db();
        let collection: Collection<DiscordServerData> = db.collection("server_data");

        let filter = doc! { "server_id": sv_id as i64 };

        let bson_value = to_bson(&value).map_err(|e| {
            Error::from(e)
        })?;

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
}

pub struct PatreonVerification {
    pub patreon_email: String,
}

impl PatreonVerification {
    pub fn new(patreon_email: String) -> Self {
        Self { patreon_email }
    }

    pub async fn verify(&self) -> Result<bool, Error> {

        let db = DatabaseManager::get_db();

        let collection: Collection<PatreonUserData> = db.collection("user_data");

        let verify = collection.find_one(doc! { "patreon_email": self.patreon_email.clone() }).await?;

        Ok(verify.is_some())
    
    }
}

#[derive(Serialize, Deserialize,Debug)]
pub struct DiscordServerData {
    verify_role_id: u64,
}

pub struct DiscordServerDatabaseManager {
}

impl DiscordServerDatabaseManager {

    pub async fn get_verify_roles() -> Option<u64> {
        let db = DatabaseManager::get_db();

        let collection: Collection<DiscordServerData> = db.collection("server_data");

        let data = collection.find_one(doc! {}).await;

        match data {
            Ok(data) => {
                if let Some(data) = data {
                    return Some(data.verify_role_id);
                }
            }
            Err(_) => {
                tracing::error!("Verify role not found")
            
        }
    }
        None
    }

    pub async fn set_verify_roles(sv_id: u64,id: u64) -> Result<(), Error> {

        DatabaseManager::update_discord_server_data(sv_id,"verify_role_id",id).await?;
        Ok(())
    }

    pub async fn set_logging_channel(sv_id: u64,channel_id: u64) -> Result<(),Error> {

        DatabaseManager::update_discord_server_data(sv_id, "log_channel_id", channel_id).await?;
        Ok(())

    }
    
}
