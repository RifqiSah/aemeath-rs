use std::time::Duration;

use poise::serenity_prelude as serenity;
use poise::serenity_prelude::gateway::ActivityData;
use rand::seq::IndexedRandom as _;
use tokio::time;

use crate::bot::bot::{Data, Error};

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub async fn ready_handler(
  ctx: &serenity::Context,
  event: &serenity::FullEvent,
  _: &Data,
) -> Result<(), Error> {
  match event {
    serenity::FullEvent::Ready { data_about_bot, ..  } => {
      let shard_id: i32 = data_about_bot.shard.map(|s| s.id.0 as i32).unwrap_or(-1);
      tracing::info!(shard = shard_id, "bot has started");

      let activities= vec![
        format!("Running build #{VERSION}"),
        format!("Watching Gaia Proxmox Clusters"),
      ];

      let mut interval = time::interval(Duration::from_secs(60));
      let ctx_clone = ctx.clone();
      tokio::spawn(async move {
        loop {
          interval.tick().await;
          let activity_name = activities.choose(&mut rand::rng()).unwrap();
          ctx_clone.set_activity(Some(ActivityData::playing(activity_name)));
        }
      });

      // command registration
      register_commands(&ctx).await;
    }
    _ => {}
  }
  Ok(())
}

async fn register_commands(ctx: &serenity::Context) {
  // Instant command registration
  #[cfg(debug_assertions)]
  for guild_id in ctx.cache.guilds() {
    tracing::debug!(guild = %guild_id, "registering command");

    if let Err(e) = guild_id.set_commands(
      &ctx.http,
      &poise::builtins::create_application_commands(&crate::bot::commands::all_commands())
    ).await {
      tracing::warn!(guild = %guild_id, error = ?e, "unable to register command");
    }
  }

  // Global registration
  #[cfg(not(debug_assertions))]
  if let Err(e) = poise::builtins::register_globally(
    &ctx.http,
    &crate::all_commands()
  ).await {
    tracing::warn!(error = ?e, "unable to register global commands");
  }
}