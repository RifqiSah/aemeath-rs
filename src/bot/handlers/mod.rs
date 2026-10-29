use poise::serenity_prelude as serenity;
use tokio::try_join;

use crate::bot::bot::Data;

pub mod error;
pub mod ready;
pub mod resume;

async fn all_handlers(
  ctx: &serenity::Context,
  event: &serenity::FullEvent,
  data: &Data,
) -> Result<(), Box<dyn std::error::Error + std::marker::Send + Sync>> {
  try_join!(
    ready::ready_handler(ctx, event, data),
    resume::resume_handler(ctx, event, data),
  )?;

  Ok(())
}

pub struct Handler;

#[serenity::async_trait]
impl serenity::EventHandler for Handler {
  async fn dispatch(&self, ctx: &serenity::prelude::Context, event: &serenity::FullEvent) {
    if let Err(err) = all_handlers(&ctx, &event, ctx.data_ref()).await {
      tracing::error!("Event handler error: {:?}", err);
    }
  }
}
