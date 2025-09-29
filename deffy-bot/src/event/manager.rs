use serenity::{
    all::{Context, Interaction, UserId},
    async_trait,
};
use tokio::sync::mpsc;

use crate::event::event_router::EVENT_ROUTER;

#[derive(Clone)]
pub enum EventData {
    Ready(serenity::model::prelude::Ready),
    Interaction(serenity::model::prelude::Interaction),
    Message(serenity::model::prelude::Message),
}

#[async_trait]
pub trait Hookable: Sync + Send + 'static {
    async fn call(&self, event: &str, ctx: Context, data: EventData) -> Result<(), anyhow::Error>;
    fn route(&self) -> Option<&'static str>;
}

inventory::collect!(&'static dyn Hookable);

pub fn spawn_event_dispatcher(mut rx: mpsc::Receiver<(String, Context, EventData)>) {
    tokio::spawn(async move {
        while let Some((event_name, ctx, data)) = rx.recv().await {
            for handler in inventory::iter::<&dyn Hookable> {
                let route_opt = handler.route();
                if let Some(route) = route_opt {
                    if let EventData::Interaction(interaction) = &data {
                        if let Some(user) = extract_user_id(interaction) {
                            if !EVENT_ROUTER.check_gateway(route, &user) {
                                continue;
                            }
                        }
                    }
                }
                if let Err(err) = handler.call(&event_name, ctx.clone(), data.clone()).await {
                    tracing::error!("[Event Error] {}: {:?}", event_name, err);
                }
            }
        }
    });
}

pub struct MasterHandler {
    pub tx: mpsc::Sender<(String, Context, EventData)>,
}

#[serenity::async_trait]
impl serenity::prelude::EventHandler for MasterHandler {
    async fn ready(&self, ctx: Context, data: serenity::model::prelude::Ready) {
        if let Err(e) = self
            .tx
            .send(("ready".into(), ctx, EventData::Ready(data)))
            .await
        {
            tracing::error!("Send error: {}", e);
        }
    }

    async fn interaction_create(&self, ctx: Context, data: serenity::model::prelude::Interaction) {
        if let Err(e) = self
            .tx
            .send((
                "interaction_create".into(),
                ctx,
                EventData::Interaction(data),
            ))
            .await
        {
            tracing::error!("Send error: {}", e);
        }
    }

    async fn message(&self, ctx: Context, data: serenity::model::prelude::Message) {
        if let Err(e) = self
            .tx
            .send(("message".into(), ctx, EventData::Message(data)))
            .await
        {
            tracing::error!("Send error: {}", e);
        }
    }
}

fn extract_user_id(interaction: &Interaction) -> Option<UserId> {
    interaction
        .as_message_component()
        .map(|c| c.user.id.clone())
        .or_else(|| interaction.as_command().map(|c| c.user.id.clone()))
        .or_else(|| interaction.as_modal_submit().map(|c| c.user.id.clone()))
}
