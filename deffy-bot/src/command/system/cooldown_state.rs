
use std::{collections::HashMap, time::{Duration, Instant}};
use tokio::sync::Mutex;

pub struct CooldownState {
    user_cooldowns: Mutex<HashMap<u64, Instant>>,
}

impl CooldownState {
    pub fn new() -> Self {
        Self {
            user_cooldowns: Mutex::new(HashMap::new()),
        }
    }

    pub async fn check_and_update(&self, user_id: u64, cooldown: Duration) -> Result<(), Duration> {
        let mut map = self.user_cooldowns.lock().await;
        let now = Instant::now();

        if let Some(last) = map.get(&user_id) {
            let elapsed = now.duration_since(*last);
            if elapsed < cooldown {
                return Err(cooldown - elapsed);
            }
        }

        map.insert(user_id, now);
        Ok(())
    }
}
