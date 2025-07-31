
use deffy_bot_macro::event;
use serenity::{all::{Context, CreateInteractionResponse, CreateInteractionResponseMessage, RoleId}};

use crate::event::manager::EventData;


#[event(e = interaction_create)]
async fn on_message(ctx: Context, data: EventData) {
    if let EventData::Interaction(interaction) = data {
        if let Some(btn) = &interaction.as_message_component() {
            match btn.data.custom_id.as_str() {
                "btn1" => {
                    let client_has_role = btn.user.has_role(&ctx.http, btn.guild_id.unwrap(), RoleId::new(1400089824471548035)).await;

                    let mut  content = format!("You don't have Role!");

                    match client_has_role {

                        Ok(client) => {
                            if client {
                                content = format!("You have Role!");
                            }
                        }
                        Err(e) => {
                            tracing::error!("{e}");
                        }
                        
                    }

                    let response = btn.create_response(
                        &ctx.http, 
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .content(content)
                                .ephemeral(true)
                        )
                    ).await;

                    if let Err(e) = response {
                        tracing::error!("{}",e)
                    }

                }
                _ => {

                }
                
            }
            
        }
    }
}