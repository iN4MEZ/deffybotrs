use std::{any::Any, sync::{Arc, Mutex}};

use handler_macro::event;
use serenity::all::Context;

#[event(e = ready)]
async fn on_ready(ctx: Context, _data: Arc<Mutex<Box<dyn Any + Send + Sync>>>) {
    tracing::info!("Logged in as {}", ctx.cache.current_user().name);
}