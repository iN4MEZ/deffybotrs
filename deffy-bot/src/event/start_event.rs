use deffy_bot_macro::event;
use once_cell::sync::OnceCell;
use serenity::all::{Context, GuildId, Http};
use std::{collections::HashMap, env, sync::Arc};
use tokio::sync::{Mutex, mpsc};

pub static BOT_HTTP: OnceCell<Arc<Http>> = OnceCell::new();

use crate::{
    command::{handler::moderator_command::BanSession, system::manager::{spawn_command_worker, CommandJob, CommandManager}},
    event::manager::EventData, session::manager::UserSessionManager,
};

pub static COMMAND_MANAGER: OnceCell<Arc<Mutex<CommandManager>>> = OnceCell::new();

#[event(e = ready)]
async fn on_ready(ctx: Context, _data: EventData) -> Result<(), Error> {
    let guild_id = GuildId::new(
        env::var("GUILD_ID")
            .expect("Expected GUILD_ID in environment")
            .parse()
            .expect("GUILD_ID must be an integer"),
    );

    UserSessionManager::new(&ctx).await;

    let (tx, rx) = mpsc::channel::<CommandJob>(100);

    spawn_command_worker(rx).await;

    // สร้าง Manager และ register
    let mut manager = CommandManager::new(tx);
    manager.register_commands();

    let commands = manager.get_commands();

    // ใส่ลง Arc<Mutex> เพื่อให้ทั่วระบบ access ได้
    let manager_arc = Arc::new(Mutex::new(manager));
    if let Err(_) = COMMAND_MANAGER.set(manager_arc.clone()) {
        tracing::error!("Failed to set command manager");
    }

    let commands = guild_id.set_commands(&ctx.http, commands).await;

    match commands {
        Ok(_) => tracing::trace!("Commands registered successfully"),
        Err(e) => tracing::error!("Failed to register commands: {}", e),
    }

    BOT_HTTP.set(ctx.http.clone()).ok();

    {
        let mut data = ctx.data.write().await;
        data.insert::<BanSession>(Arc::new(Mutex::new(HashMap::new())));
    }

    tracing::info!("Logged in as {}", &ctx.cache.current_user().name);

    Ok(())
}