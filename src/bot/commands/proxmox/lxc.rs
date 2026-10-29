use crate::bot::bot::{Context, Error};

#[poise::command(
  slash_command,
  description_localized("en-US", "Memeriksa latency API dan latency Shorekeeper terhadap server Discord."),
  guild_only
)]
pub async fn lxc(ctx: Context<'_>) -> Result<(), Error> {
  let latency = {
    if let Some(duration) = ctx.ping() {
      format!("{:.2}", duration.as_millis())
    } else {
      "???".to_string()
    }
  };

  ctx.reply(format!("🏓 Pong! Latency **{}ms**", latency)).await?;

  Ok(())
}