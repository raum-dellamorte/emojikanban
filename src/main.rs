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
    let access_token = serve_oauth_receiver()?;
    println!("Twitch access token: {access_token}");
    EkbConfigUpdate {
      oauth: Some(access_token),
      ..Default::default()
    }
  } else {
    EkbConfigUpdate::default()
  };
  
  let runtime = tokio::runtime::Runtime::new().unwrap();
  let (oauth_tx, mut oauth_rx) = tokio::sync::mpsc::unbounded_channel();
  let _handle = runtime.spawn(async {
    emojikanban::get_or_create_config_emojikanban(config_update, oauth_tx).await;
  });
  let (ekb_config_dirs, conf) = match oauth_rx.blocking_recv() {
    Some(emojikanban::plugin::TwitchOAuthRcvr::NewConfigData(data)) => data,
    Some(emojikanban::plugin::TwitchOAuthRcvr::OAuthToken(_)) => { panic!("Got token not asked for in main") }
    Some(emojikanban::plugin::TwitchOAuthRcvr::RcvrError(e)) => { panic!("Error getting config in main: {}", e) }
    None => { unreachable!() }
  };
  let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<emojikanban::EmoteComEnum>();
  runtime.spawn(async move {
    emojikanban::start_twitch_monitor(ekb_config_dirs, conf, tx).await;
  });
  while let Some(emote_data) = rx.blocking_recv() {
    match emote_data {
      emojikanban::EmoteComEnum::Chat(chat_msg) => {
        for emote_data in chat_msg.emotes.iter() {
          println!("Emote :{}: used.", emote_data.name);
        }
      }
      emojikanban::EmoteComEnum::SqliteConnectionFailure(e) => {
        // let e = e.clone();
        // let err = e.as_ref();
        // let error = e.as_ref().as_ref().unwrap_err();
        log::error!("Failed to connect Sqlite: {}", e.as_ref().as_ref().unwrap_err());
      }
      emojikanban::EmoteComEnum::TwitchConnectionFailure(e) => {
        log::error!("Twitch monitor died: {}", e.as_ref().as_ref().unwrap_err());
      }
    }
  }
  
  Ok(())
}
