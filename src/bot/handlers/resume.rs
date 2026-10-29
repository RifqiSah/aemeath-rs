use std::time::Duration;

use poise::serenity_prelude::{self as serenity};

use crate::bot::bot::{Data, Error};

const POST_RESUME_CHECK_DELAY_SECS: u64 = 10;
const HEALTH_CHECK_TIMEOUT_SECS: u64 = 15;

pub async fn resume_handler(
  ctx: &serenity::Context,
  event: &serenity::FullEvent,
  data: &Data,
) -> Result<(), Error> {
  match event {
    serenity::FullEvent::Resume { event, .. } => {
      tracing::warn!(event = ?event, "bot has resumed after {POST_RESUME_CHECK_DELAY_SECS}s");

      let http = ctx.http.clone();
      let restart_tx = data.client_restart_tx.clone();

      tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(POST_RESUME_CHECK_DELAY_SECS)).await;

        tracing::debug!("post-resume: pinging Discord API to verify session validity...");

        let result = tokio::time::timeout(
          Duration::from_secs(HEALTH_CHECK_TIMEOUT_SECS),
          http.get_current_user(),
        )
        .await;

        match result {
          // Session is valid -- Discord recognized us
          Ok(Ok(user)) => {
            tracing::info!("post-resume health check passed: session valid (bot: {}).", user.name);
          }

          // Request reached Discord but was rejected -- session invalid
          Ok(Err(e)) => {
            tracing::error!(error = ?e, "post-resume health check failed: Discord rejected the request. session is likely invalid. triggering client restart.");
            let _ = restart_tx.send(());
          }

          // Request never got a response within the timeout window
          Err(_timeout) => {
            tracing::error!("post-resume health check TIMED OUT after {HEALTH_CHECK_TIMEOUT_SECS}s. Network issue or dead session. triggering client restart.");
            let _ = restart_tx.send(());
          }
        }
      });
    }

    _ => {}
  }

  Ok(())
}