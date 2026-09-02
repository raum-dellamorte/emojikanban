use {
  crate::{
    config_kdl::*,
    plugin::*,
    // TwitchOAuthRcvr::*,
  },
  anyhow::{
    Result,
    anyhow,
  },
  cosmic_text::Color,
  futures::StreamExt,
  irc::client::prelude::*,
  kdl::{
    KdlDocument,
    KdlError,
  },
  obs_wrapper::{
    module::{
      LoadContext,
      Module,
      ModuleRef,
    },
    obs_register_module,
    obs_string,
    obs_sys::{
      obs_frontend_add_tools_menu_item,
      obs_frontend_open_source_properties,
      obs_source_create_private,
    },
    source::SourceRef,
    string::ObsString,
    wrapper::PtrWrapper,
  },
  platform_dirs::AppDirs,
  rusqlite::{
    Connection, 
    params,
  },
  std::{
    ffi::c_void,
    panic::{
      AssertUnwindSafe, catch_unwind,
    },
    path::PathBuf,
    sync::{
      Arc, /*Mutex,*/ OnceLock,
    },
  },
  tokio::{
    runtime::{
      Handle, Runtime,
    },
    sync::{
      broadcast, mpsc, watch,
    },
    task::JoinHandle,
  },
  twitch_api::{
    helix::HelixClient, 
    twitch_oauth2::{
      AccessToken, 
      UserToken, 
    },
  },
};

pub mod config_kdl;
pub mod effects;
pub mod font_studio;
pub mod plugin;

const PROMOTE_DEBUG_LOGS: bool = false;

pub static EKB_BROADCAST: OnceLock<EkbBroadcast> = OnceLock::new();

pub fn ekb_broadcast() -> &'static EkbBroadcast {
  EKB_BROADCAST.get().unwrap()
}

#[derive(Debug)]
pub struct EkbBroadcast {
  pub runtime: Handle,
  pub chat_tx: broadcast::Sender<Arc<ChatData>>,
  pub cmd_tx: mpsc::UnboundedSender<TwitchMgrCmd>,
  pub cfg_rx: watch::Receiver<Option<EkbConfigSnapshot>>,
}

struct EKBModule {
  ctx: ModuleRef,
  runtime: Option<Runtime>,
  twitch_mgr_handle: Option<JoinHandle<()>>,
  settings_source: Option<SourceRef>,
  tools_menu_state: Option<Box<ToolsMenuState>>,
}

impl Drop for EKBModule {
  fn drop(&mut self) {
    if let Some(runtime) = self.runtime.take() {
      runtime.shutdown_timeout(std::time::Duration::from_millis(100));
    }
  }
}

impl Module for EKBModule {
  fn new(ctx: ModuleRef) -> Self {
    // Start the logger
    let _ = obs_wrapper::log::Logger::new().with_promote_debug(PROMOTE_DEBUG_LOGS).init();
    // Launch a tokio runtime
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (chat_tx, _) = broadcast::channel::<Arc<ChatData>>(256);
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
    let (cfg_tx, cfg_rx) = watch::channel::<Option<EkbConfigSnapshot>>(None);
    let twitch_mgr_handle = Some(runtime.spawn(twitch_connection_mgr(
      cmd_rx, chat_tx.clone(), cfg_tx,
    )));
    EKB_BROADCAST.set(
      EkbBroadcast {
        runtime: runtime.handle().clone(),
        chat_tx,
        cmd_tx,
        cfg_rx,
      }
    ).unwrap();
    let runtime = Some(runtime);
    Self { ctx, runtime, twitch_mgr_handle, settings_source: None, tools_menu_state: None, }
  }
  fn get_ctx(&self) -> &ModuleRef {
    &self.ctx
  }
  fn load(&mut self, load_context: &mut LoadContext) -> bool {
    let mut settings_info = load_context
      .create_source_builder::<EkbSettings>()
      .enable_get_name()
      .enable_get_properties()
      .enable_update()
      .build();
    settings_info.as_mut().output_flags |= obs_wrapper::obs_sys::OBS_SOURCE_CAP_DISABLED;
    load_context.register_source(settings_info);
    let raw_settings_source = unsafe {
      obs_source_create_private(
        obs_string!("emojikanban_settings").as_ptr(),
        obs_string!("EmojiKanBan Configuration").as_ptr(),
        std::ptr::null_mut(),
      )
    };
    let Some(settings_source) = (unsafe {
        SourceRef::from_raw_unchecked(raw_settings_source)
    }) else {
      log::error!("Failed to create private EmojiKanBan Settings source.");
      return false;
    };
    let menu_state = Box::new(ToolsMenuState {
      settings_source: settings_source.clone(), 
    });
    self.settings_source = Some(settings_source);
    let private_data = menu_state.as_ref() as *const ToolsMenuState as *mut c_void;
    unsafe {
      obs_frontend_add_tools_menu_item(
        obs_string!("EmojiKanBan Configuration").as_ptr(),
        Some(open_ekb_config),
        private_data
      );
    }
    self.tools_menu_state = Some(menu_state);
    let emojikanban_info = load_context
      .create_source_builder::<EmojiKanBan>()
      .enable_get_name()
      .enable_get_properties()
      .enable_get_width()
      .enable_get_height()
      .enable_update()
      .enable_video_render()
      .enable_video_tick()
      .build();
    load_context.register_source(emojikanban_info);
    let chatto_source = load_context
      .create_source_builder::<ChattoKanBan>()
      .enable_get_name()
      .enable_get_properties()
      .enable_get_width()
      .enable_get_height()
      .enable_update()
      .enable_video_render()
      .enable_video_tick()
      .build();
    load_context.register_source(chatto_source);
    true
  }
  fn unload(&mut self) {
    if let Some(handle) = self.twitch_mgr_handle.take() {
      handle.abort();
    }
    self.settings_source.take();
    self.tools_menu_state.take();
    if let Some(runtime) = self.runtime.take() {
      runtime.shutdown_timeout(std::time::Duration::from_millis(100));
    }
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

struct ToolsMenuState {
  settings_source: SourceRef,
}

unsafe extern "C" fn open_ekb_config(private_data: *mut c_void) {
  if private_data.is_null() {
    log::error!("EmojiKanBan Tools Menu callback received null data.");
    return;
  }
  let result = catch_unwind(AssertUnwindSafe(|| {
    let state = unsafe {
      &*(private_data as *const ToolsMenuState)
    };
    unsafe {
      obs_frontend_open_source_properties(state.settings_source.as_ptr_mut());
    }
  }));
  if result.is_err() {
    log::error!("EmojiKanBan Tools Menu callback panicked.");
  }
}

pub async fn twitch_connection_mgr(
  mut cmd_rx: mpsc::UnboundedReceiver<TwitchMgrCmd>,
  chat_tx: broadcast::Sender<Arc<ChatData>>,
  cfg_tx: watch::Sender<Option<EkbConfigSnapshot>>,
) {
  let mut config_update = EkbConfigUpdate::default();
  loop {
    let (config_dirs, config) = match get_or_create_config_emojikanban(config_update).await {
      Ok(config) => config,
      Err(error) => {
        log::error!(
          "Failed to load EmojiKanBan configuration: {}",
          error,
        );
        match cmd_rx.recv().await {
          Some(TwitchMgrCmd::UpdateConfig(update)) => {
            config_update = update;
            continue;
          }
          Some(TwitchMgrCmd::Reconnect) => {
            config_update = EkbConfigUpdate::default();
            continue;
          }
          Some(TwitchMgrCmd::Shutdown) | None => { return; },
        }
      }
    };
    cfg_tx.send_replace(Some(EkbConfigSnapshot::from(&config)));
    let mut monitor = tokio::spawn(
      start_twitch_monitor(config_dirs, config, chat_tx.clone(), )
    );
    tokio::select! {
      result = &mut monitor => {
        match result {
          Ok(Ok(())) => {
            log::warn!("Twitch monitor stopped");
          }
          Ok(Err(error)) => {
            log::error!("Twitch monitor failed: {}", error);
          }
          Err(error) => {
            log::error!(
              "Twitch monitor task failed: {}",
              error,
            );
          }
        }
        // Add a reconnect delay here.
        tokio::time::sleep(
          std::time::Duration::from_secs(5),
        ).await;
        config_update = EkbConfigUpdate::default();
      }
      command = cmd_rx.recv() => {
        monitor.abort();
        match command {
          Some(TwitchMgrCmd::UpdateConfig(update)) => {
            config_update = update;
          }
          Some(TwitchMgrCmd::Reconnect) => {
            config_update = EkbConfigUpdate::default();
          }
          Some(TwitchMgrCmd::Shutdown) | None => return,
        }
      }
    }
  }
}

pub async fn start_twitch_monitor(mut ekb_conf_dirs: EkbConfigDirs, conf: EkbTwitchConfig, chat_tx: broadcast::Sender<Arc<ChatData>>) -> anyhow::Result<()> {
  let emotes = match connect_sqlite(&mut ekb_conf_dirs) {
    Ok(emotes) => emotes,
    Err(e) => {
      // _ = tx.send(Arc::new(EmoteComEnum::SqliteConnectionFailure(Err(e.into()))));
      return Err(e.into());
    }
  };
  let mut client = match connect_twitch_client(&conf).await {
    Ok(client) => { client }
    Err(e) => {
      // _ = tx.send(Arc::new(EmoteComEnum::TwitchConnectionFailure(Err(e.into()))));
      return Err(e.into());
    }
  };
  let mut stream = match client.stream() {
    Ok(client) => { client }
    Err(e) => {
      // _ = tx.send(Arc::new(EmoteComEnum::TwitchConnectionFailure(Err(e.into()))));
      return Err(e.into());
    }
  };
  loop {
    let irc_response = stream.next().await.transpose();
    if irc_response.is_err() {
      return Err(anyhow!("IRC error {}", irc_response.unwrap_err()));
    }
    let irc_response = irc_response.unwrap();
    if irc_response.is_none() {
      return Err(anyhow!("Twitch IRC stream ended"));
    }
    match irc_response.unwrap().to_twitch_message_privmsg() {
      Err(_msg) => {
        // Do something with this?
      }
      Ok(pm) => {
        let user: String = pm.display_name().unwrap_or("Anonymous").to_owned();
        let chat_msg: String = pm.data.to_owned().into();
        let ColorConverter::<Color>(uname_color) = pm.color().into();
        let mut chat_data = ChatData { user, msg: chat_msg, uname_color, emotes: Vec::new() };
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
                loc: emote.byte_pos,
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
              loc: emote.byte_pos,
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
          // let _ = tx.send(EmoteComEnum::Data(emote_data));
          chat_data.emotes.push(emote_data);
        }
        chat_data.emotes.sort_by_key(|e| e.loc.0 );
        let _ = chat_tx.send(Arc::new(chat_data));
      }
    }
  }
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

fn connect_sqlite(path: &mut EkbConfigDirs) -> Result<Connection, rusqlite::Error> {
  if path.data.is_file() { path.data.pop(); }
  path.data.push("emotes.db3");
  let db = Connection::open(&mut path.data)?;
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
pub async fn get_or_create_config_emojikanban(config_update: EkbConfigUpdate) -> anyhow::Result<(EkbConfigDirs, EkbTwitchConfig)> {
  let app_name = Some("emojikanban");
  let config_file = "config.kdl";
  let config_kdl = DEFAULT_CONFIG_KDL;
  let app_dirs = AppDirs::new(app_name, true).ok_or_else(|| {
    anyhow!("Failed to get home directory. Cannot check for or create config file.")
  })?;
  let config_dir = app_dirs.config_dir;
  let data_dir = app_dirs.data_dir;
  // let mut cache_dir = app_dirs.cache_dir;
  std::fs::create_dir_all(&config_dir).map_err(|e| {
    anyhow!("Failed to create config dir: {}\nError: {}", config_dir.display(), e)
  })?;
  std::fs::create_dir_all(&data_dir).map_err(|e|  {
    anyhow!("Failed to create data dir: {}\nError: {}", data_dir.display(), e)
  })?;
  let config_path = config_dir.join(config_file);
  let config_exists = config_path.try_exists().map_err(|e| {
    anyhow!("Failed to check existence of config file: {}\nError: {}", config_path.display(), e)
  })?;
  if !config_exists {
    std::fs::write(&config_path, config_kdl).map_err(|e| {
      anyhow!("Failed to write default config file: {}\nError: {}", config_path.display(), e)
    })?;
    return Err(anyhow!(
      "Default config.kdl created at {}\n
      In OBS Studio, click `Tools -> EmojiKanBan Configuration`\n
      to open the Properties window and enter your connection\n
      details, then attempt to reconnect.", config_path.display()
    ));
  }
  // The file exists, now we need to validate it
  let config_string = std::fs::read_to_string(&config_path).map_err(|e| {
    anyhow!("File exists but failed to read: {}\nError: {}", config_path.display(), e)
  })?;
  validate_config(config_path, data_dir, config_string, config_update).await
}

#[allow(clippy::needless_return, unused)]
async fn validate_config(mut config_path: PathBuf, data_path: PathBuf, conf: String, config_update: EkbConfigUpdate) -> Result<(EkbConfigDirs, EkbTwitchConfig)> {
  let mut doc_res: Result<KdlDocument, KdlError> = conf.parse();
  let mut write_changes = false;
  match doc_res {
    Err(e) => {
      let error = anyhow!("Failed to parse {}\nError: {}", config_path.display(), e);
      // log::error!("{}", error);
      return Err(error.into());
    }
    Ok(mut doc) => {
      if let Some(new_value) = config_update.bot_account {
        match doc.bot_account_update(&new_value) {
          Ok(_) => { write_changes = true; }
          Err(e) => { log::error!("kdl update error for bot-account: {}", e); }
        }
      };
      if let Some(new_value) = config_update.channel {
        match doc.channel_update(&new_value) {
          Ok(_) => { write_changes = true; }
          Err(e) => { log::error!("kdl update error for channel: {}", e); }
        }
      }
      if let Some(new_value) = config_update.oauth {
        match doc.oauth_update(&new_value) {
          Ok(_) => { write_changes = true; }
          Err(e) => { log::error!("kdl update_error for oauth: {}", e); }
        }
      }
      let client: HelixClient<reqwest::Client> = HelixClient::default();
      match EkbTwitchConfig::try_from(doc.clone()) {
        Err(e) => {
          let error = anyhow!("Failed to parse {}\nError: {}", config_path.display(), e);
          log::error!("{}", error);
          return Err(error);
        }
        Ok(conf) => {
          let token = AccessToken::new(conf.oauth());
          match UserToken::from_token(&client, token.clone()).await {
            Err(e) => {
              let error = anyhow!("Failed to validate oauth token: {:?}, {}", conf, e);
              log::error!("{}", error);
              return Err(error);
            }
            Ok(token) => {
              let bot_account = conf.bot_account();
              let channel = conf.channel();
              let bot_valid = client.get_channel_from_login(&bot_account, &token).await
                .map_err(|e| { anyhow!("Failure awaiting client.get_channel_from_login for bot account. {}", e) });
              let chn_valid = client.get_channel_from_login(&channel, &token).await
                .map_err(|e| { anyhow!("Failure awaiting client.get_channel_from_login for streamer channel. {}", e) });
              if bot_valid.is_ok() && bot_valid.as_ref().unwrap().is_some() && chn_valid.is_ok() && chn_valid.as_ref().unwrap().is_some() {
                if write_changes && let Err(e) = std::fs::write(&config_path, doc.to_string()) {
                  log::error!("Failed to write new values to {}\nValues will not be retained after this session.\nError: {}", config_path.display(), e);
                }
                config_path.pop();
                return Ok((EkbConfigDirs{ config: config_path, data: data_path}, conf));
              } else {
                let error = anyhow!(
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

#[derive(Clone)]
pub struct EmoteData {
  pub id: String,
  pub name: String,
  pub img: Vec<u8>,
  pub loc: (usize, usize),
}

#[derive(Clone)]
pub struct ChatData {
  pub user: String,
  pub msg: String,
  pub uname_color: Option<Color>,
  pub emotes: Vec<EmoteData>
}

pub enum TwitchMgrCmd {
  UpdateConfig(EkbConfigUpdate),
  Reconnect,
  Shutdown,
}

pub struct ColorConverter<T>(Option<T>);

impl From<Option<twitch_message::Color>> for ColorConverter<Color> {
  fn from(value: Option<twitch_message::Color>) -> Self {
    if let Some(c) = value {
      ColorConverter(Some(Color::rgb(c.0, c.1, c.2)))
    } else { ColorConverter(None) }
  }
}