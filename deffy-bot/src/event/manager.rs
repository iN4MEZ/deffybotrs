
use serenity::{all::Context, async_trait};
use tokio::sync::mpsc;

#[derive(Clone)]
pub enum EventData {
    Ready(serenity::model::prelude::Ready),
    Interaction(serenity::model::prelude::Interaction),
    Message(serenity::model::prelude::Message),
}


#[async_trait]
pub trait Hookable: Sync + Send + 'static {
    async fn call(&self, event: &str, ctx: Context, data: EventData);
}

inventory::collect!(&'static dyn Hookable);

pub async fn spawn_event_dispatcher(
    mut rx: mpsc::Receiver<(String, Context, EventData)>,
) {
    tokio::spawn(async move {
        while let Some((event_name, ctx, data)) = rx.recv().await {
            for handler in inventory::iter::<&dyn Hookable> {
                handler.call(&event_name, ctx.clone(), data.clone()).await;
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
        let _ = self
            .tx
            .send(("ready".into(), ctx, EventData::Ready(data)))
            .await;
    }

    async fn interaction_create(&self, ctx: Context, data: serenity::model::prelude::Interaction) {
        let _ = self
            .tx
            .send((
                "interaction_create".into(),
                ctx,
                EventData::Interaction(data),
            ))
            .await;
    }

    async fn message(&self, ctx: Context, data: serenity::model::prelude::Message) {
        let _ = self
            .tx
            .send(("message".into(), ctx, EventData::Message(data)))
            .await;
    }
}