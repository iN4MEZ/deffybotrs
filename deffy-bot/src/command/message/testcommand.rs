use std::{any::Any, sync::{Arc, Mutex}};

use serenity::{all::{Context, CreateInteractionResponse, CreateInteractionResponseMessage}, async_trait};

use crate::command::command_registry::{CommandHandler, CommandInfo};

pub struct TestCommand;

#[async_trait]
impl CommandHandler for TestCommand {

    async fn run(&self, ctx: Context,data: Arc<Mutex<Box<dyn Any + Send + Sync>>>) {
        tracing::info!("TestCommand executed");
        if let Ok(mut locked_data) = data.lock() {
            if let Some(interaction) = locked_data.downcast_mut::<serenity::model::prelude::Interaction>() {
                if let Some(response) = interaction.as_command() {
                    let response_clone = response.clone();
                    tokio::spawn(async move {
                        let ctx_clone = ctx.clone();
                        let _ = response_clone.create_response(ctx_clone.http, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().content(
                            format!("Hello, {} This is a test command response.", response_clone.user.name),
                        ))).await;
                    });
                }
            }
        }
    }
    
}
impl CommandInfo for TestCommand {
    fn name(&self) -> &'static str {
        "test"
    }

    fn description(&self) -> &'static str {
        "A test command"
    }
}

