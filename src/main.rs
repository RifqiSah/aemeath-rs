mod log;
mod bot;

use dotenvy::dotenv;

fn print_header() {
  println!(r"
                                   _   _     
    /\                            | | | |    
   /  \   ___ _ __ ___   ___  __ _| |_| |__  
  / /\ \ / _ \ '_ ` _ \ / _ \/ _` | __| '_ \ 
 / ____ \  __/ | | | | |  __/ (_| | |_| | | |
/_/    \_\___|_| |_| |_|\___|\__,_|\__|_| |_|");

  println!("\n");
  tracing::info!("aemeath started");
}

async fn wait_for_shutdown_signal() {
  #[cfg(unix)]
  {
    use tokio::signal::unix::{signal, SignalKind};

    let mut sigterm = signal(SignalKind::terminate()).unwrap();
    let mut sigint = signal(SignalKind::interrupt()).unwrap();

    tokio::select! {
      _ = sigterm.recv() => println!(""),
      _ = sigint.recv() => println!(""),
    }
  }

  #[cfg(not(unix))]
  {
    tokio::signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
  }
}

#[tokio::main]
async fn main() {
  dotenv().ok();
  
  // Global logger
  let _global_log = log::init_logger();
  
  print_header();

  let (shutdown_tx, _) = tokio::sync::broadcast::channel::<()>(1);
  let mut shutdown_rx = shutdown_tx.subscribe();

  // Initialize
  // let config = shorekeeper_config::Config::from_env().with_toml();

  // let _ = shorekeeper_infra::db::mysql::init_db_pool(&config).await;
  // let _ = shorekeeper_infra::cache::redis::Cache::init_cache_pool(&config).await;
  // let _ = shorekeeper_infra::integration::discord::api::client::init_discord_client(&config);
  
  // Bot Module
  let shutdown_bot = shutdown_tx.clone();
  // let config_bot = config.clone();
  let bot_task = tokio::spawn(async move {
    crate::bot::bot::start_bot(shutdown_bot).await;
  });
 
  // Shutdown channel
  tokio::select! {
    _ = wait_for_shutdown_signal() => {},
    _ = shutdown_rx.recv() => {}
  }

  let _ = shutdown_tx.send(());

  // Thread wait
  let _ = tokio::join!(bot_task);
}