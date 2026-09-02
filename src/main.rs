// This file is just here for testing without running the plugin.
use {
  emojikanban::{
    config_kdl::*,
  },
  std::{
    io::Write,
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
    println!("Open this URL in your browser:\n{TWITCH_AUTH_URL}");
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
  
  let runtime = tokio::runtime::Runtime::new().unwrap();
  let (_oauth_tx, mut oauth_rx) = tokio::sync::mpsc::unbounded_channel();
  let _handle = runtime.spawn(async {
    let _ = emojikanban::get_or_create_config_emojikanban(config_update).await;
  });
  let (ekb_config_dirs, conf) = match oauth_rx.blocking_recv() {
    Some(emojikanban::plugin::TwitchOAuthRcvr::NewConfigData(data)) => data,
    Some(emojikanban::plugin::TwitchOAuthRcvr::OAuthToken(_)) => { panic!("Got token not asked for in main") }
    Some(emojikanban::plugin::TwitchOAuthRcvr::RcvrError(e)) => { panic!("Error getting config in main: {}", e) }
    None => { unreachable!() }
  };
  let (chat_tx, mut rx) = tokio::sync::broadcast::channel::<std::sync::Arc<emojikanban::ChatData>>(256);
  let (cmd_tx, cmd_rx) = tokio::sync::mpsc::unbounded_channel();
  let (cfg_tx, cfg_rx) = tokio::sync::watch::channel::<Option<EkbConfigSnapshot>>(None);
  let _twitch_mgr_handle = Some(runtime.spawn(emojikanban::twitch_connection_mgr(cmd_rx, chat_tx.clone(), cfg_tx)));
  emojikanban::EKB_BROADCAST.set(
    emojikanban::EkbBroadcast {
      runtime: runtime.handle().clone(),
      chat_tx: chat_tx.clone(),
      cmd_tx,
      cfg_rx,
    }
  ).unwrap();
  runtime.spawn(async move {
    let _ = emojikanban::start_twitch_monitor(ekb_config_dirs, conf, chat_tx.clone()).await;
  });
  while let Ok(chat_msg) = rx.blocking_recv() {
    // match emote_data.as_ref() {
      // emojikanban::EmoteComEnum::Chat(chat_msg) => {
        for emote_data in chat_msg.emotes.iter() {
          println!("Emote :{}: used.", emote_data.name);
        }
      // }
      // emojikanban::EmoteComEnum::SqliteConnectionFailure(e) => {
      //   // let e = e.clone();
      //   // let err = e.as_ref();
      //   // let error = e.as_ref().as_ref().unwrap_err();
      //   log::error!("Failed to connect Sqlite: {}", e.as_ref().as_ref().unwrap_err());
      // }
      // emojikanban::EmoteComEnum::TwitchConnectionFailure(e) => {
      //   log::error!("Twitch monitor died: {}", e.as_ref().as_ref().unwrap_err());
      // }
    // }
  }
  
  Ok(())
}
