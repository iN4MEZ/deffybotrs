use deffy_bot_macro::event;
use serenity::all::{ComponentInteractionDataKind, Context};

use crate::{command::moderate_command::ACTIVE_BANS, event::manager::EventData};

#[event(e = interaction_create)]
async fn on_interaction(_ctx: Context, data: EventData) {
    if let EventData::Interaction(interaction) = data {
        if let Some(sm) = interaction.as_message_component() {
            let user_interact_id = sm.user.id;
    
            let active = ACTIVE_BANS.lock().await;
    
            if active.contains(&user_interact_id) {
                // สร้าง custom_id ที่คาดหวังทั้ง 2 แบบ
                let confirm_btn_id = format!("confirmbanbtn:{}", user_interact_id);
                let select_menu_id = format!("banuser:{}", user_interact_id);
    
                match sm.data.custom_id.as_str() {
                    id if id == confirm_btn_id => {
                        tracing::debug!("Confirm button clicked by user: {}", user_interact_id);
                        // ใส่ logic กด confirm ตรงนี้
                    }
                    id if id == select_menu_id => {
                        if let ComponentInteractionDataKind::UserSelect { values } = &sm.data.kind {
                            // values คือ Vec<UserId> ที่ user เลือกมา
                            for user_id in values {
                                tracing::debug!("User selected: {}", user_id);
                            }
                            // หรือเอาไปประมวลผลต่อได้เลย
                        }
                        
                    }
                    _ => {
                        tracing::warn!("Unknown interaction type or custom_id: {}", sm.data.custom_id);
                    }
                }
            } else {
                
            }
        }
    }
}
