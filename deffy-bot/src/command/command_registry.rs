use std::{any::Any, collections::HashMap, sync::{Arc, Mutex}};

use serenity::{all::{Context, CreateCommand}, async_trait};

#[async_trait]
pub trait CommandHandler: Send + Sync + 'static {
    async fn run(&self, ctx: Context,data: Arc<Mutex<Box<dyn Any + Send + Sync>>>);
}

pub trait CommandInfo: Send + Sync + 'static {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
}

pub trait CommandMeta: CommandHandler + CommandInfo {}
impl<T> CommandMeta for T where T: CommandHandler + CommandInfo {}

pub struct CommandManager {
    commands: Vec<CreateCommand>,
    handlers: HashMap<String, Arc<dyn CommandHandler>>,
}

impl CommandManager  {
    pub fn new() -> Self {
        Self { commands: Vec::new(), handlers: HashMap::new() }
    }

    pub fn register_command<T: CommandMeta + 'static>(&mut self, command: T) {
        let arc = Arc::new(command);
        let created_cmd = CreateCommand::new(arc.name())
            .description(arc.description())
            .clone();

        self.commands.push(created_cmd);
        self.handlers.insert(arc.name().to_owned(),arc);
        tracing::info!("Command registered: {}", std::any::type_name::<T>());
    }

    pub fn get_commands(&self) -> Vec<CreateCommand> {
        self.commands.clone()
    }

    pub fn get_handler(&self, name: &str) -> Option<Arc<dyn CommandHandler>> {
        for handler in &self.handlers {
            if handler.0 == name {
                return Some(handler.1.clone());
            }
        }
        None
    }
}