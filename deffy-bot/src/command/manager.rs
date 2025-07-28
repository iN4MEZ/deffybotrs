use std::{
    collections::HashMap,
    result,
    sync::{Arc},
};

use anyhow::Error;
use serenity::{
    all::{CommandInteraction, ComponentInteraction, Context, CreateCommand},
    async_trait,
};

#[derive(Debug, Clone)]
pub enum InteractionWrapper {
    Command(CommandInteraction),
    Component(ComponentInteraction)
}

inventory::collect!(CommandRegistration);

#[async_trait]
pub trait CommandHandler: Send + Sync + 'static + CommandInfo {
    async fn execute(
        &self,
        ctx: Context,
        interaction: CommandInteraction,
    ) -> result::Result<(), Error>;
    async fn execute_component(
        &self,
        ctx: Context,
        interaction: ComponentInteraction,
    ) -> result::Result<(), Error>;
    fn register(&self) -> CreateCommand;
}

pub trait CommandInfo: Send + Sync + 'static {
    fn name(&self) -> &'static str;
}

pub struct CommandRegistration {
    pub constructor: fn() -> Arc<dyn CommandHandler>,
}

pub struct CommandManager {
    commands: HashMap<String, (CreateCommand, Arc<dyn CommandHandler>)>,
}

impl CommandManager {
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
        }
    }

    pub fn register_commands(&mut self)
    {
        for entry in inventory::iter::<CommandRegistration> {
            let handler = (entry.constructor)();
            let name = handler.name();
            let create_command = handler.register();
            //Store the command name as a String and the handler as Arc<dyn CommandHandler>
            self.commands.insert(
                name.to_string(),
                (create_command, handler),
            );

            tracing::info!("Registered Command: {}_COMMAND", name.to_uppercase());
        }
    }

    pub fn get_commands(&self) -> Vec<CreateCommand> {
        self.commands.values().map(|(cmd, _)| cmd.clone()).collect::<Vec<_>>()
    }

    pub fn get_handler(&self, name: &str) -> Option<Arc<dyn CommandHandler>> {
        self.commands.get(name).map(|(_, handler)| handler.clone())
    }
}
