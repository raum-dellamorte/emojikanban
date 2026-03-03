use {
  crate::{
    config_kdl::*,
    plugin::*,
  },
  anyhow::Result,
  futures::StreamExt,
  irc::client::prelude::*,
  kdl::{
    KdlDocument,
    KdlError,
    // KdlValue,
  },
  obs_wrapper::{
    module::{
      LoadContext,
      Module,
      ModuleRef,
    },
    obs_register_module,
    obs_string,
    string::ObsString
  },
  platform_dirs::AppDirs,
  rusqlite::{
    Connection, 
    params,
  },
  std::path::PathBuf,
  tokio::sync::mpsc::UnboundedSender,
  twitch_api::{
    helix::HelixClient, 
    twitch_oauth2::{
      AccessToken, 
      UserToken, 
    }
  },
  // twitch_message::{
  //   // IntoStatic,
  //   messages::{
  //     Message as TwitchMsg,
  //     MessageKind,
  //     Privmsg,
  //   },
  // },
};

mod config_kdl;
pub mod effects;
pub mod plugin;

struct EKBModule {
  ctx: ModuleRef,
}

impl Module for EKBModule {
  fn new(ctx: ModuleRef) -> Self {
    Self { ctx }
  }
  fn get_ctx(&self) -> &ModuleRef {
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
    match irc_response.to_twitch_message_privmsg() {
      Err(_msg) => {
        // Do something with this?
      }
      Ok(pm) => {
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
      }
    }
  }
  Ok(())
}

trait ToTwitchMessagePrivmsg: Sized {
  fn to_twitch_message_privmsg(self) -> Result<twitch_message::messages::Privmsg<'static>, Self>;
}

impl ToTwitchMessagePrivmsg for irc::proto::Message {
  fn to_twitch_message_privmsg(self) -> Result<twitch_message::messages::Privmsg<'static>, Self> {
    // Fix for single emote not being detected.
    // Provided by [museun](https://github.com/museun)
    // 
    // Chat messages with only a single word or emote are incorrectly encoded
    // by `irc = "1.1.0"`. It fails to preceed the chat data with a colon in
    // that case. So we skip their .to_string() implementation and convert
    // directly to `twitch_message::messages::Privmsg`
    let irc::proto::Command::PRIVMSG(target, data) = &self.command else {
      return Err(self);
    };
    use twitch_message::builders::{PrivmsgBuilder, TagsBuilder};
    let mut privmsg_builder = PrivmsgBuilder::new().channel(target).data(data);
    if let Some(sender) = self.source_nickname() {
      privmsg_builder = privmsg_builder.sender(sender);
    }
    let mut tags_builder = TagsBuilder::default();
    if let Some(tags) = &self.tags {
      for irc::proto::message::Tag(key, value) in tags {
        tags_builder = tags_builder.add(key, value.as_deref().unwrap_or(""));
      }
    }
    privmsg_builder
      .tags(tags_builder.finish())
      .finish_privmsg()
      .map_err(|_| self)
  }
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

async fn connect_twitch_client(conf: &EkbTwitchConfig) -> Result<irc::client::Client, irc::error::Error> {
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
pub async fn get_or_create_config_emojikanban() -> Result<(PathBuf, EkbTwitchConfig), String> {
  let app_name = Some("emojikanban");
  let config_file = "config.kdl";
  let config_kdl = 
r#"bot-account bot-name                       // <- Replace 'bot-name' with the name of the account used to monitor chat
channel     streamer-name                  // <- and 'streamer-name' with the streamer, most likely your own
oauth       g0Bble0dEE0GukK0enCryPTIon0KEy // <- With or without "oauth:" prefix
// The oauth should be generated from the account you use
// as the 'bot-account'. If you use your streamer account,
// you can use the same account name for 'bot-account' and
// 'channel'. 'channel' is only used to select the irc channel to 
// monitor for emotes, and eventually for chat.
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
async fn validate_config(mut config_path: PathBuf, conf: String) -> Result<(PathBuf, EkbTwitchConfig), String> {
  let conf = &conf;
  let doc: Result<KdlDocument, KdlError> = conf.parse();
  match doc {
    Err(e) => {
      let error = format!("Failed to parse {}\nError: {}", config_path.display(), e);
      log::error!("{}", error);
      return Err(error);
    }
    Ok(conf) => {
      let client: HelixClient<reqwest::Client> = HelixClient::default();
      match EkbTwitchConfig::try_from(conf) {
        Err(e) => {
          let error = format!("Failed to parse {}\nError: {}", config_path.display(), e);
          log::error!("{}", error);
          return Err(error);
        }
        Ok(conf) => {
          let token = AccessToken::new(conf.oauth());
          match UserToken::from_token(&client, token.clone()).await {
            Err(e) => {
              let error = format!("Failed to validate oauth token: {:?}, {}", conf, e);
              log::error!("{}", error);
              return Err(error);
            }
            Ok(token) => {
              let bot_account = conf.bot_account();
              let channel = conf.channel();
              let bot_valid = client.get_channel_from_login(&bot_account, &token).await
                .expect("Failure awaiting client.get_channel_from_login for bot account.");
              let chn_valid = client.get_channel_from_login(&channel, &token).await
                .expect("Failure awaiting client.get_channel_from_login for streamer channel.");
              if bot_valid.is_some() && chn_valid.is_some() {
                config_path.pop();
                return Ok((config_path, conf));
              } else {
                let error = format!(
                  "OAUTH Token valid, but either the bot_username or the channel is invalid in: {}\nbot-account: {} {:?}\nchannel: {} {:?}",
                  config_path.display(), bot_account, bot_valid, channel, chn_valid, 
                );
                log::error!("{}", error);
                return Err(error);
              }
            }
          }
        }
      }
    }
  }
}

