// This file is just here for testing without running the plugin.

fn main() -> Result<(), anyhow::Error> {
  env_logger::Builder::from_default_env()
    .filter(None, log::LevelFilter::Info)
    .init();
  let runtime = tokio::runtime::Runtime::new().unwrap();
  let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<emojikanban::plugin::EmoteData>();
  runtime.spawn(async move {
    if let Err(e) = emojikanban::run(tx).await {
      log::error!("Twitch monitor died: {}", e);
    };
  });
  while let Some(emote_data) = rx.blocking_recv() {
    println!("Emote :{}: used.", emote_data.name);
  }
  
  Ok(())
}

