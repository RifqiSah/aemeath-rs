use std::collections::HashMap;
use std::str::FromStr as _;
use std::sync::Arc;
use std::time::SystemTime;

use poise::serenity_prelude as serenity;
use poise::serenity_prelude::GatewayIntents;
use tokio::sync::Mutex;
use tokio::time::{Duration, sleep};

use super::commands::all_commands;
use super::handlers::Handler;
use super::handlers::error;

pub struct Data {
  #[allow(dead_code)]
  pub start_time: SystemTime,
  #[allow(dead_code)]
  pub shutdown_tx: tokio::sync::broadcast::Sender<()>,
  #[allow(dead_code)]
  pub pending_forward: Mutex<HashMap<u64, serenity::Message>>,
  pub client_restart_tx: tokio::sync::broadcast::Sender<()>,
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

async fn make_client(
  token: &str,
  intents: GatewayIntents,
  shutdown_tx: tokio::sync::broadcast::Sender<()>,
  client_restart_tx: tokio::sync::broadcast::Sender<()>,
) -> Result<serenity::Client, Box<dyn std::error::Error + Send + Sync>> {
  //
  let Ok(token) = serenity::Token::from_str(&token) else {
    tracing::error!("unable to parse Discord token");
    return Err("Unable to parse Discord token".into());
  };

  let options = poise::FrameworkOptions {
    commands: all_commands(),
    // pre_command: |ctx| {
    //   Box::pin(async move {
    //     tracing::debug!("Executing command {}...", ctx.command().qualified_name);
    //   })
    // },
    // post_command: |ctx| {
    //   Box::pin(async move {
    //     tracing::debug!("Executed command {}!", ctx.command().qualified_name);
    //   })
    // },
    initialize_owners: true,
    on_error: |error| {
      Box::pin(async move {
        if let Err(e) = error::on_error(error).await {
          tracing::error!(error = ?e, "error while handling error");
        }
      })
    },
    prefix_options: poise::PrefixFrameworkOptions {
      mention_as_prefix: false,
      ..Default::default()
    },
    ..Default::default()
  };

  let framework = poise::Framework::builder()
    .options(options)
    .build();

  let client = serenity::ClientBuilder::new(token, intents)
    .framework(Box::new(framework))
    .event_handler(Arc::new(Handler {}))
    .data(Arc::new({
      Data {
        start_time: SystemTime::now(),
        shutdown_tx: shutdown_tx.clone(),
        client_restart_tx: client_restart_tx.clone(),
        pending_forward: Mutex::new(HashMap::new()),
      }
    }))
    .await
    .expect("Unable to create client instance!");

  Ok(client)
}

fn watchdog(
  client: &serenity::Client,
  sleep_duration: Duration,
  client_restart_tx: tokio::sync::broadcast::Sender<()>,
) {
  let runner_watchdog = client.shard_manager.runners.clone();
  let mut zero_latency_streak: u32 = 0;

  tokio::spawn(async move {
    loop {
      sleep(sleep_duration).await;

      if client_restart_tx.receiver_count() == 0 {
        tracing::debug!("watchdog: no restart receivers left, exiting.");
        return;
      }

      let mut should_restart = false;

      for entry in runner_watchdog.iter() {
        let (id, runner) = entry.pair();
        let info = runner.info.read();
        let latency = info.latency.unwrap_or_default();

        tracing::debug!(shard = %id, stage = ?info.stage, latency_ms = latency.as_secs_f64() * 1_000.0, "shard status");

        match info.stage {
          serenity::ConnectionStage::Connecting | serenity::ConnectionStage::Disconnected => {
            tracing::warn!(shard = %id, stage = ?info.stage, "triggering client restart");
            should_restart = true;
            break;
          }

          serenity::ConnectionStage::Connected => {
            if latency.is_zero() {
              zero_latency_streak += 1;
              tracing::warn!(shard = %id, streak = zero_latency_streak, "shard connected but latency is 0ms, possible dead connection");

              if zero_latency_streak >= 2 {
                tracing::error!(shard = %id, streak = zero_latency_streak, "shard has 0ms latency for consecutive watchdog cycles, riggering client restart");
                
                should_restart = true;
                break;
              }
            } else {
              zero_latency_streak = 0;
            }
          }

          _ => {
            zero_latency_streak = 0;
          }
        }
      }

      if should_restart {
        let _ = client_restart_tx.send(());
        drop(runner_watchdog);
        return;
      }
    }
  });
}

pub async fn start_bot(shutdown_tx: tokio::sync::broadcast::Sender<()>) {
  rustls::crypto::ring::default_provider()
    .install_default()
    .expect("Failed to install rustls crypto provider");

  let mut shutdown_rx = shutdown_tx.subscribe();

  let token = std::env::var("APP_DISCORD_TOKEN").expect("Missing APP_DISCORD_TOKEN");
  let intents = GatewayIntents::GUILDS
    | GatewayIntents::GUILD_MESSAGES
    | GatewayIntents::GUILD_MESSAGE_REACTIONS
    | GatewayIntents::GUILD_PRESENCES
    | GatewayIntents::GUILD_MEMBERS
    | GatewayIntents::GUILD_WEBHOOKS
    | GatewayIntents::GUILD_VOICE_STATES
    | GatewayIntents::MESSAGE_CONTENT
    | GatewayIntents::DIRECT_MESSAGES
    | GatewayIntents::DIRECT_MESSAGE_TYPING;

  let bot_task = tokio::spawn({
    let token = token.clone();
    let shutdown_tx = shutdown_tx.clone();
    let mut shutdown_rx_inner = shutdown_tx.subscribe();

    async move {
      loop {
        let (client_restart_tx, mut client_restart_rx) = tokio::sync::broadcast::channel::<()>(1);

        tokio::select! {
          _ = shutdown_rx_inner.recv() => {
            // shutdown signal
            tracing::warn!("bot task received shutdown signal, exiting loop");
            break;
          }

          _ = async {
            // Create client framework
            let client_result = make_client(
              &token,
              intents,
              shutdown_tx.clone(),
              client_restart_tx.clone(),
            ).await;

            let mut client = match client_result {
              Ok(c) => c,
              Err(e) => {
                tracing::error!(error = ?e, "error creating bot client, retrying in 5 seconds ...");
                sleep(Duration::from_secs(5)).await;

                return;
              }
            };

            watchdog(&client, Duration::from_secs(300), client_restart_tx.clone());

            // Starting client
            let runners_manager = client.shard_manager.runners.clone();
            tracing::info!("starting bot client");

            tokio::select! {
              // Watchdog or resume_handler triggered a client-only restart
              _ = client_restart_rx.recv() => {
                tracing::warn!("client restart triggered. dropping client and restarting in 5 seconds...");

                drop(runners_manager);
                drop(client);

                sleep(Duration::from_secs(5)).await;
              }

              // Client exited on its own (clean or error)
              result = client.start() => {
                match result {
                  Ok(()) => {
                    tracing::warn!("bot client exited cleanly (unexpected). restarting in 5 seconds...");
                  }
                  Err(e) => {
                    tracing::error!(error = ?e, "bot client error, restarting in 5 seconds ...");
                  }
                }
                
                drop(runners_manager);
                sleep(Duration::from_secs(5)).await;

                tracing::info!("retrying bot client start ...");
              }
            }
          } => {}
        }
      }
    }
  });

  let _ = shutdown_rx.recv().await;
  tracing::warn!("received shutdown signal, exiting");

  let _ = bot_task.await;
}