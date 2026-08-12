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
  let oauth: Option<String> = if answer.trim().eq_ignore_ascii_case("y") {
    println!("Open this URL in your browser:\n{TWITCH_AUTH_URL}");
    let access_token = serve_oauth_receiver()?;
    println!("Twitch access token: {access_token}");
    Some(access_token)
  } else {
    None
  };
  
  let runtime = tokio::runtime::Runtime::new().unwrap();
  let (ekb_config_dirs, conf) = runtime.block_on(async {
    match emojikanban::get_or_create_config_emojikanban(oauth).await {
      Err(e)  => { Err(anyhow::format_err!("{}", e)) }
      Ok(res) => { Ok(res) }
    }
  })?;
  
  let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<emojikanban::EmoteComEnum>();
  runtime.spawn(async move {
    emojikanban::start_twitch_monitor(ekb_config_dirs, conf, tx).await;
  });
  while let Some(emote_data) = rx.blocking_recv() {
    match emote_data {
      emojikanban::EmoteComEnum::Data(emote_data) => {
        println!("Emote :{}: used.", emote_data.name);
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
