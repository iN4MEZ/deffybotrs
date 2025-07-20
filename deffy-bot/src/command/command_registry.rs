use std::{result, sync::Arc};

use serenity::{all::{Context, CreateCommand, Interaction}, async_trait};

#[async_trait]
pub trait CommandHandler: Send + Sync + 'static + CommandInfo {
    async fn execute(&self, ctx: Context,data: Interaction) -> result::Result<(), std::io::Error>;
    fn register(&self) -> CreateCommand;
}

pub trait CommandInfo: Send + Sync + 'static {
    fn name(&self) -> &'static str;
}

pub struct CommandManager {
    commands: Vec<(CreateCommand,Arc<dyn CommandHandler>)>
}

impl CommandManager  {
    pub fn new() -> Self {
        Self { commands: Vec::new() }
    }

    pub fn register_command<T>(&mut self, command: T)
    where
        T: CommandHandler,
    {
        let arc = Arc::new(command);

        let created_cmd = arc.register();
        self.commands.push((created_cmd, arc.clone()));

        tracing::info!("Command registered: {}", std::any::type_name::<T>());
    }

    pub fn get_commands(&self) -> Vec<CreateCommand> {
        self.commands.iter().map(|(cmd, _)| cmd.clone()).collect()
    }

    pub fn get_handler(&self, name: &str) -> Option<Arc<dyn CommandHandler>> {
        for handler in &self.commands {
            if handler.1.name() == name {
                return Some(handler.1.clone());
            }
        }
        None
    }
}