use std::{any::Any, sync::{Arc, Mutex}};

use serenity::{all::Context, async_trait};

#[async_trait]
pub trait Hookable: Sync + Send + 'static {
    async fn call(&self, event: &str, ctx: Context, data: Arc<Mutex<Box<dyn Any + Send>>>);
    fn event_type(&self) -> &'static str;
}

inventory::collect!(&'static dyn Hookable);

pub struct MasterHandler;

#[serenity::async_trait]
impl serenity::prelude::EventHandler for MasterHandler {
    async fn ready(&self, ctx: Context, data: serenity::model::prelude::Ready) {

        let data = Arc::new(Mutex::new(Box::new(data) as Box<dyn Any + Send>));
        for handler in inventory::iter::<&dyn Hookable> {
            let data_clone = Arc::clone(&data);
            handler.call("ready", ctx.clone(), data_clone).await;
        }
    }

    async fn interaction_create(&self, ctx: Context, data: serenity::model::prelude::Interaction) {
        let shared_data = Arc::new(Mutex::new(Box::new(data) as Box<dyn Any + Send>));
        for handler in inventory::iter::<&dyn Hookable> {
            let data_clone = Arc::clone(&shared_data);
            handler.call("interaction_create", ctx.clone(), data_clone).await;
        }
    }

    // ... เพิ่ม event อื่นตามที่ต้องการ
}
