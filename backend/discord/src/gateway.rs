//! The gateway connection that shows the bot as online.

use twilight_gateway::{EventTypeFlags, Intents, Shard, ShardId, StreamExt as _};

use crate::bot::Discord;

impl Discord {
    /// Holds the bot's gateway connection open so Discord shows it online. Reconnects and resumes
    /// by itself; returns only when Discord closes it for good, such as for a bad token.
    pub async fn run_gateway(&self) {
        let token = self.config().bot_token.as_str().to_owned();
        let mut shard = Shard::new(ShardId::ONE, token, Intents::empty());
        while let Some(event) = shard.next_event(EventTypeFlags::READY).await {
            match event {
                Ok(_) => tracing::info!("discord gateway ready"),
                Err(error) => tracing::warn!(%error, "discord gateway error"),
            }
        }
        tracing::error!("discord gateway closed for good; the bot will show offline");
    }
}
