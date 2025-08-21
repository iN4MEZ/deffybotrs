use std::sync::Arc;

use axum::{body::Bytes, extract::State, http::{HeaderMap, StatusCode}, response::IntoResponse, routing::post, Router};
use deffy_bot_patreon_services::{Event, Webhook};
use deffy_bot_utils::event::manager::EVENT_MANAGER;

pub async fn routes() -> Router {

    let webhook_secret = std::env::var("PATREON_WEBHOOK_SECRET".to_string())
        .expect("PATREON_WEBHOOK_SECRET must be set in the environment variables");
    
    let webhook = Arc::new(Webhook {
        webhook_secret: webhook_secret,
    });

    Router::new().route("/", post(root).with_state(webhook))
}

async fn root(
    State(state): State<Arc<Webhook>>,
    headers: HeaderMap,
    body: Bytes
) -> impl IntoResponse {
    let signature = headers
        .get("x-patreon-signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let trigger = headers
        .get("x-patreon-event")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    match (signature.is_empty(), trigger.is_empty()) {
        (true, _) => return (StatusCode::BAD_REQUEST, "Missing signature header"),
        (_, true) => return (StatusCode::BAD_REQUEST, "Missing event header"),
        _ => {}
    }

    // match state.check_signature(&body, signature) {
    //     Ok(true) => tracing::trace!("Signature is valid"),
    //     Ok(false) => {
    //         tracing::error!("Invalid signature: {}", signature);
    //         return (StatusCode::UNAUTHORIZED, "Invalid signature");
    //     }
    //     Err(e) => {
    //         tracing::error!("Error checking signature: {:?}", e);
    //         return (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error");
    //     }
    // }

    match state.parse_event(&body, trigger) {
        Ok(event) => {
            match event {
                Event::CreateMember(member) => {
                    tracing::trace!("🎉 New member joined: {:?}", member);

                    EVENT_MANAGER.lock().await.emit(deffy_bot_utils::event::manager::EventType::PatreonWebhookUserCreated,deffy_bot_utils::event::manager::EventTypeData::PatreonData(member.attributes.full_name)).await;
                    
                }
                Event::UpdateMember(member) => {
                    tracing::trace!("🔄 Member updated: {:?}", member);
                }
                Event::DeleteMember(member) => {
                    tracing::trace!("❌ Member left: {:?}", member);
                }
                _ => tracing::trace!("ℹ️ Other event: {:?}", event),
            }
            (StatusCode::OK, "OK")
        }
        Err(e) => {
            tracing::error!("⚠️ Error parsing event: {:?}", e);
            (StatusCode::BAD_REQUEST, "Invalid event")
        }
    }
}
