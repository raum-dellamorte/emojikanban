// This file is just here for testing without running the plugin.
use {
  emojikanban::{
    ChatData, TwitchMgrCmd,
    config_kdl::*,
    twitch_connection_mgr,
  },
  std::{
    io::Write,
    sync::Arc,
  },
  tokio::{
    sync::{
      broadcast, mpsc, watch,
    }
  },
};

fn main() -> Result<(), anyhow::Error> {
  env_logger::Builder::from_default_env()
    .filter(None, log::LevelFilter::Info)
    .init();
  print!("Update Twitch authentication? (y/N) ");
  std::io::stdout().flush()?;
  let mut answer = String::new();
  std::io::stdin().read_line(&mut answer)?;
  let config_update: EkbConfigUpdate = if answer.trim().eq_ignore_ascii_case("y") {
    let listener = std::net::TcpListener::bind("127.0.0.1:3000")?;
    if let Err(e) = open::that(TWITCH_CALLBACK_URL) {
      log::error!("Failed to open {}. Error: {}", TWITCH_CALLBACK_URL, e);
    }
    let access_token = serve_oauth_receiver(listener)?;
    println!("Twitch access token: {access_token}");
    EkbConfigUpdate {
      oauth: Some(access_token),
      ..Default::default()
    }
  } else {
    EkbConfigUpdate::default()
  };
  let runtime = tokio::runtime::Runtime::new()?;
  let (chat_tx, mut chat_rx) = broadcast::channel::<Arc<ChatData>>(256);
  let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<TwitchMgrCmd>();
  let (cfg_tx, mut cfg_rx) = watch::channel::<Option<EkbConfigSnapshot>>(None);
  if config_update.oauth.is_some()
  || config_update.bot_account.is_some()
  || config_update.channel.is_some()
  {
    cmd_tx.send(TwitchMgrCmd::UpdateConfig(config_update))?;
  }
  let manager_handle = runtime.spawn(twitch_connection_mgr(
    cmd_rx, chat_tx, cfg_tx,
  ));
  let config_handle = runtime.spawn(async move {
    while cfg_rx.changed().await.is_ok() {
      let snapshot = cfg_rx.borrow_and_update().clone();
      if let Some(snapshot) = snapshot {
        log::info!(
          "Connected as '{}' to channel '{}'",
          snapshot.bot_account,
          snapshot.channel,
        );
      }
    }
  });
  loop {
    match chat_rx.blocking_recv() {
      Ok(chat) => {
        println!("{}: {}", chat.user, chat.msg);
        for emote in &chat.emotes {
          println!("  Emote :{}: at bytes {:?}", emote.name, emote.loc);
        }
      }
      Err(broadcast::error::RecvError::Lagged(skipped)) => {
        log::warn!("Skipped {} stale chat messages", skipped);
      }
      Err(broadcast::error::RecvError::Closed) => {
        log::warn!("Chat broadcast closed.");
        break;
      }
    }
  }
  // These lines are normally reached only if
  // the manager exits and closes the chat broadcast.
  let _ = cmd_tx.send(TwitchMgrCmd::Shutdown);
  manager_handle.abort();
  config_handle.abort();
  Ok(())
}
