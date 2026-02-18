use {
  crate::{
    confparse::*,
    // effects::*,
    plugin::{
      *,
    },
  },
  futures::StreamExt, 
  irc::client::prelude::*, 
  obs_wrapper::{
    obs_register_module,
    obs_string,
    module::{
      LoadContext,
      Module,
      ModuleContext,
    },
    string::ObsString,
  },
  platform_dirs::AppDirs,
  rusqlite::{
    Connection, 
    Result, 
    params
  }, 
  std::{
    path::PathBuf,
  }, 
  tokio::sync::mpsc::UnboundedSender,
  twitch_api::{
    helix::HelixClient, 
    twitch_oauth2::{
      AccessToken, 
      UserToken, 
    }
  }, 
  twitch_message::messages::{
    Message as TwitchMsg,
    MessageKind,
    Privmsg,
  },
};

mod confparse;
pub mod effects;
pub mod plugin;

struct EKBModule {
  ctx: ModuleContext,
}

impl Module for EKBModule {
  fn new(ctx: ModuleContext) -> Self {
    Self { ctx }
  }
  fn get_ctx(&self) -> &ModuleContext {
    &self.ctx
  }
  fn load(&mut self, load_context: &mut LoadContext) -> bool {
    let source = load_context
      .create_source_builder::<EmojiKanBan>()
      .enable_get_name()
      .enable_get_properties()
      .enable_get_width()
      .enable_get_height()
      .enable_update()
      .enable_video_render()
      .enable_video_tick()
      .build();
    load_context.register_source(source);
    true
  }
  fn unload(&mut self) {
    // I assume cleanup goes here but I don't know that there's anything to clean up
  }
  fn post_load(&mut self) {
    // I assume that if further setup were needed after load, that would go here
  }
  fn description() -> ObsString {
    obs_string!("Emote Wall and on-screen chat source for OBS.")
  }
  fn name() -> ObsString {
    obs_string!("libemojikanban")
  }
  fn author() -> ObsString {
    obs_string!("Raum Dellamorte")
  }
}

obs_register_module!(EKBModule);

pub async fn run(tx: UnboundedSender<EmoteData>) -> Result<(), anyhow::Error> {
  let (mut config_path, conf) = match get_or_create_config_emojikanban().await {
    Err(e) => { return Err(anyhow::format_err!("{}", e)); }
    Ok(res) => { res }
  };
  let emotes = connect_sqlite(&mut config_path)?;
  let mut client = connect_twitch_client(&conf).await?;
  let mut stream = client.stream()?;
  while let Some(irc_response) = stream.next().await.transpose()? {
    let irc_msg = irc_response.to_string();
    let result = twitch_message::parse(&irc_msg)?;
    let msg: TwitchMsg<'_> = result.message;
    if let MessageKind::Privmsg = msg.kind && let Some(pm) = msg.as_typed_message::<Privmsg>() {
      for emote in pm.emotes() {
        let uri_v1 = format!("https://static-cdn.jtvnw.net/emoticons/v1/{}/3.0", emote.id);
        let uri_v2 = format!("https://static-cdn.jtvnw.net/emoticons/v2/{}/default/light/3.0", emote.id);
        // println!("Emote URI: {}", uri_v1);
        let emote_data: EmoteData = if let Ok(emote_data) = emotes.query_one(
          "SELECT id, name, img FROM emotes WHERE id=?1", params![emote.id.to_string()], |row| {
            Ok(EmoteData{
              id: row.get(0)?,
              name: row.get(1)?,
              img: row.get(2)?,
            })
          })
        {
          log::info!("Loaded emote id {} from sqlite", emote.id);
          emote_data
        } else {
          log::info!("Could not find id {} in DB, downloading image to DB...", emote.id);
          let img_data = if let Ok(data) = reqwest::get(uri_v2).await {
            data 
          } else if let Ok(data) = reqwest::get(uri_v1).await { data } else {
            log::error!("Failed to download image data for emote id {} at step 1", emote.id);
            continue;
          };
          let img_bytes = if let Ok(bytes) = img_data.bytes().await { bytes } else {
            log::error!("Failed to download image data for emote id {} at step 2", emote.id);
            continue;
          };
          if image::load_from_memory(&img_bytes).is_err() {
            log::error!("Failed to validate downloaded image data for emote id {}\n  data: {:?}", emote.id, img_bytes);
            continue;
          }
          let emote_data = EmoteData{
            id: emote.id.to_string(),
            name: emote.name.into_owned(), // FixMe: this sometimes ends up with several names, probably when multiple emotes are used in the same chat
            img: img_bytes.into(),
          };
          if let Err(e) = emotes.execute(
            "INSERT INTO emotes (id, name, img) VALUES (?1, ?2, ?3)",
            params![emote_data.id.clone(), emote_data.name.clone(), emote_data.img.clone()],
          ) {
            log::error!("Failed to write emote to DB: {}", e)
          };
          log::info!("Loaded emote id {} from URI", emote.id);
          emote_data
        };
        let _ = tx.send(emote_data);
      }
    };
  }
  Ok(())
}

fn connect_sqlite(path: &mut PathBuf) -> Result<Connection, rusqlite::Error> {
  if path.is_file() { path.pop(); }
  path.push("emotes.db3");
  let db = Connection::open(path)?;
  if let Ok(false) = db.table_exists(None, "emotes") {
    db.execute(
      "CREATE TABLE emotes (
          id   VARCHAR(255) PRIMARY KEY,
          name VARCHAR(255),
          img  BLOB NOT NULL
      )",
      (),
    )?;
  }
  Ok(db)
}

async fn connect_twitch_client(conf: &EkbConfig) -> Result<irc::client::Client, irc::error::Error> {
  let config = Config {
    nickname: Some(conf.bot_account()),
    server: Some("irc.chat.twitch.tv".to_owned()),
    port: Some(6697_u16),
    use_tls: Some(true),
    channels: vec![format!("#{}", conf.channel())],
    password: Some(format!("oauth:{}", conf.oauth())),
    ..Default::default()
  };
  let client = irc::client::Client::from_config(config).await?;
  client.send(Command::Raw("CAP REQ :twitch.tv/tags twitch.tv/commands twitch.tv/membership".to_owned(), vec![]))?;
  client.identify()?;
  Ok(client)
}

#[allow(clippy::needless_return)] // 'return' statements make the intention more obvious.
pub async fn get_or_create_config_emojikanban() -> Result<(PathBuf, EkbConfig), String> {
  let app_name = Some("emojikanban");
  let config_file = "config.kdl";
  let config_kdl = 
r#"bot-account bot-name                       // <- Replace 'bot-name' with the name of the account used to monitor chat
channel     streamer-name                  // <- and 'streamer-name' with the streamer, most likely your own
oauth       g0Bble0dEE0GukK0enCryPTIon0KEy // <- With or without "oauth:" prefix
// The oauth should be generated from the account you use
// as the 'bot-account'. If you use your streamer account,
// you should be able to use the same account name for 
// 'bot-account' and 'channel', but I don't know for sure.
// 'channel' is only used to select the irc channel to 
// monitor for emotes, and eventually for chat.
// 
// !!! This is not a real .kdl file. !!! The parser expects 
// bot-account on the first line, channel on the second line, 
// and oauth on the third line, each followed by whitespace 
// then by a string of non-space characters as the value. 
// Any whitespace after the value marks the beginning of a
// comment till the end of the line such that text on the 
// same line after the value is ignored. The '//' are there 
// for decoration, even in this block because parsing stops 
// after the oauth line's value.
// 
// key    value   This text is ignored with or without '//'
// 
"#;
  if let Some(app_dirs) = AppDirs::new(app_name, true) {
    let mut path = app_dirs.config_dir;
    if let Err(e) = std::fs::create_dir_all(&path) {
      let error = format!("Failed to create config dir: {}\nError: {}", path.display(), e);
      // log::error!("{}", error);
      return Err(error);
    }
    path.push(config_file);
    match std::fs::exists(&path) {
      Err(e)    => {
        let error = format!("Failed to check existence of config file: {}\nError: {}", path.display(), e);
        // log::error!("{}", error);
        return Err(error);
      }
      Ok(false) => {
        if let Err(e) = std::fs::write(&path, config_kdl) {
          let error = format!("Failed to write default config file: {}\nError: {}", path.display(), e);
          // log::error!("{}", error);
          return Err(error);
        } else {
          let error = format!("Default config.kdl created at {}", path.display());
          // log::info!("{}", error);
          return Err(error);
        }
      }
      Ok(true)  => {
        // The file exists, now we need to validate it
        match std::fs::read_to_string(&path) { 
          Err(e) => {
            let error = format!("File exists but failed to read: {}\nError: {}", path.display(), e);
            // log::error!("{}", error);
            return Err(error);
          }
          Ok(conf) => {
            return validate_config(path, conf).await;
          }
        }
      }
    }
  } else {
    let error = "Failed to get home directory. Cannot check for or create config file.".to_owned();
    log::info!("{}", error);
    return Err(error)
  }
}

#[allow(clippy::needless_return)]
async fn validate_config(mut config_path: PathBuf, conf: String) -> Result<(PathBuf, EkbConfig), String> {
  match parse_config(&conf) {
    Err(e) => {
      let error = format!("Failed to parse {}\nError: {}", config_path.display(), e);
      log::error!("{}", error);
      return Err(error);
    }
    Ok(conf) => {
      let client: HelixClient<reqwest::Client> = HelixClient::default();
      let mut oauth = conf.oauth();
      if oauth.len() >= 6 && &oauth[..6] == "oauth:" {
        oauth = oauth[6..].to_owned();
      }
      let token = AccessToken::new(oauth);
      match UserToken::from_token(&client, token.clone()).await {
        Err(e) => {
          let error = format!("Failed to validate oauth token: {:?}, {}", conf, e);
          log::error!("{}", error);
          return Err(error);
        }
        Ok(token) => {
          let bot_valid = client.get_channel_from_login(&conf.bot_account(), &token).await
            .expect("Failure awaiting client.get_channel_from_login for bot account.");
          let chn_valid = client.get_channel_from_login(&conf.channel(), &token).await
            .expect("Failure awaiting client.get_channel_from_login for streamer channel.");
          if bot_valid.is_some() && chn_valid.is_some() {
            config_path.pop();
            return Ok((config_path, conf));
          } else {
            let error = format!(
              "OAUTH Token valid, but either the bot_username or the channel is invalid in: {}\nbot-account: {} {:?}\nchannel: {} {:?}",
              config_path.display(), conf.bot_account(), bot_valid, conf.channel(), chn_valid, 
            );
            log::error!("{}", error);
            return Err(error);
          }
        }
      }
    }
  }
}

