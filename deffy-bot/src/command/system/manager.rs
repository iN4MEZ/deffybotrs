use std::{
    collections::HashMap,
    result,
    sync::{Arc},
};

use anyhow::Error;
use serenity::{
    all::{CommandInteraction, Context, CreateCommand},
    async_trait,
};
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct CommandJob {
    pub ctx: Context,
    pub interaction: CommandInteraction,
    pub handler: Arc<dyn CommandHandler>,
}

inventory::collect!(CommandRegistration);

#[async_trait]
pub trait CommandHandler: Send + Sync + 'static + CommandInfo {
    async fn execute(
        &self,
        ctx: Context,
        interaction: CommandInteraction,
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
    pub tx: tokio::sync::mpsc::Sender<CommandJob>,
}

impl CommandManager {
    pub fn new(tx: mpsc::Sender<CommandJob>) -> Self {
        Self {
            commands: HashMap::new(),
            tx,
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

pub async fn spawn_command_worker(mut rx: tokio::sync::mpsc::Receiver<CommandJob>) {
    tokio::spawn(async move {
        while let Some(job) = rx.recv().await {
            let CommandJob {
                ctx,
                interaction,
                handler,
            } = job;

            tokio::spawn(async move {
                if let Err(err) = handler.execute(ctx, interaction).await {
                    tracing::error!("Command execution failed: {:?}", err);
                }
            });
        }
    });
}
