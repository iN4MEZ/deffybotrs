use std::{any::Any, sync::{Arc, Mutex}};

use handler_macro::event;
use serenity::all::Context;

#[event(e = ready)]
async fn on_ready(ctx: Context, _data: Arc<Mutex<Box<dyn Any + Send + Sync>>>) {
    // let data = data.lock().unwrap();
    // if let Some(ready) = data.downcast_ref::<serenity::model::prelude::Ready>() {
    //     tracing::info!("Bot is ready: {}", ready.user.name);
    // } else {
    //     tracing::warn!("Data is not of type Ready");
    // }
    tracing::info!("Logged in as {}", ctx.cache.current_user().name);
}