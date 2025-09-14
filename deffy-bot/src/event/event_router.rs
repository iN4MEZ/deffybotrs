use std::{collections::HashMap, sync::{Arc, Mutex}};
use once_cell::sync::Lazy;
use serenity::all::UserId;

enum EventRoute {
    ModerateEvent(String), 
}

// global singleton
pub static EVENT_ROUTER: Lazy<EventRouter> = Lazy::new(EventRouter::new);

#[derive(Clone)]
pub struct EventRouter {
    routes: Arc<Mutex<HashMap<UserId,EventRoute>>>,
}

impl EventRouter {
    pub fn new() -> Self {
        Self {
            routes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn register(&self, router_id: impl Into<String>, user: &UserId) {
        self.routes.lock().unwrap().insert(*user, EventRoute::ModerateEvent(router_id.into()));

        tracing::info!("Registered route for user: {}", user);
        
    }

    pub fn unregister(&self, router_id: &str, user: UserId) {
        let mut routes = self.routes.lock().unwrap();
        if let Some(route) = routes.get(&user) {
            match route {
                EventRoute::ModerateEvent(rid) if rid == router_id => {
                    routes.remove(&user);
                    tracing::info!("Unregistered route for user: {}", user);
                },
                _ => {
                    tracing::warn!("No matching route found for user: {}", user);
                }
            }
        } else {
            tracing::warn!("No route found for user: {}", user);
        }

    }

    pub fn check_gateway(&self, router_id: &str, user: &UserId) -> bool {
        let routes = self.routes.lock().unwrap();
        if let Some(route) = routes.get(user) {

            tracing::info!("Checking route for user: {}", user);

            match route {
                EventRoute::ModerateEvent(rid) if rid == router_id => true,
                _ => false,
            }
        } else {
            false
        }
    }

    // pub fn dispatch(&self, ctx: Context, interaction: Interaction) {
    //     if let Some((router_id, user)) = extract_router(&interaction) {
    //         if let Some(handler) = self.routes.lock().unwrap().get(&(router_id.clone(), user)) {
    //             handler(ctx, interaction);
    //         }
    //     }
    // }
}