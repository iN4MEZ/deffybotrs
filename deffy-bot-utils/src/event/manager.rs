use std::{collections::HashMap, sync::Arc};

use once_cell::sync::Lazy;
use serenity::async_trait;
use tokio::sync::Mutex;

#[derive(Clone)]
pub enum EventTypeData {
    PatreonData(String)
}

inventory::collect!(&'static dyn EventInfo);

pub static EVENT_MANAGER: Lazy<Mutex<EventManager>> = Lazy::new(|| Mutex::new(EventManager::new()));

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub enum EventType {
    PatreonWebhookUserCreated,
    BotStarted,
}

pub struct EventManager {
    event_handlers: Arc<Mutex<HashMap<EventType, Vec<Arc<dyn UtilsEventHandler>>>>>,
}

#[async_trait]
pub trait UtilsEventHandler: Send + Sync + 'static {
    async fn handle(&self,data: EventTypeData) -> Result<(), anyhow::Error>;
}

pub trait EventInfo: Send + Sync + 'static {
    fn event_type(&self) -> EventType;
    fn boxed(&self) -> Arc<dyn UtilsEventHandler>;
}

impl EventManager {
    pub fn new() -> Self {
        EventManager {
            event_handlers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn register(&mut self) {
        for event_info in inventory::iter::<&'static dyn EventInfo> {
            self.register_event(event_info.event_type(), event_info.boxed())
                .await;
        }
    }

    pub async fn register_event(&mut self, event: EventType, handler: Arc<dyn UtilsEventHandler>) {
        let mut handlers = self.event_handlers.lock().await;

        tracing::debug!("Registering handler for event: {:?}", event);

        handlers.entry(event).or_default().push(handler);
    }

    pub async fn emit(&self, event_type: EventType,data: EventTypeData) {
        let handler_list = {
            let map: tokio::sync::MutexGuard<
                '_,
                HashMap<EventType, Vec<Arc<dyn UtilsEventHandler + 'static>>>,
            > = self.event_handlers.lock().await;
            map.get(&event_type).cloned()
        };

        if let Some(handlers) = handler_list {
            for handler in handlers {
                let data = data.clone();

                tokio::spawn(async move {
                    if let Err(err) = handler.handle(data).await {
                        tracing::error!("Handler error: {:?}", err);
                    }
                });
            }
        } else {
            tracing::warn!("No handlers registered for event: {:?}", event_type);
        }
    }
}
