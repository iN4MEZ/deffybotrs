use mongodb::{Collection, bson::doc};
use serde::{Deserialize, Serialize};

use crate::database::DatabaseManager;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WipEntry {
    pub title: String,
    pub channel_id: u64,
    pub description: String, // Optional field for description
    pub message_id: u64,
    pub image: String, // Optional field for image URL
    pub state: u8, // 1-6
}

pub struct WipDatabase {}

impl WipDatabase {
    pub async fn create_wip(entry: WipEntry) -> Result<(), mongodb::error::Error> {
        let db = DatabaseManager::get_db();
        let collection: Collection<WipEntry> = db.collection("wip_data");

        collection.insert_one(entry).await?;
        Ok(())
    }

    pub async fn update_wip(entry: WipEntry) -> Result<(), mongodb::error::Error> {
        let db = DatabaseManager::get_db();
        let collection: Collection<WipEntry> = db.collection("wip_data");

        let filter = doc! { "message_id": entry.message_id as i64 };

        let update = doc! {
            "$set": {
                "state": entry.state as i64,
            }
        };

        collection.update_one(filter, update).await?;

        Ok(())
    }

    pub async fn get_wip(title: &str) -> Result<Option<WipEntry>, mongodb::error::Error> {
        let db = DatabaseManager::get_db();
        let collection: Collection<WipEntry> = db.collection("wip_data");

        let filter = doc! { "title": title };

        let wip = collection.find_one(filter).await?;

        Ok(wip)
    }

    pub async fn remove_wip(title: &str) -> Result<(), mongodb::error::Error> {
        let db = DatabaseManager::get_db();
        let collection: Collection<WipEntry> = db.collection("wip_data");

        let filter = doc! { "title": title };

        collection.delete_one(filter).await?;

        Ok(())
    }
}
