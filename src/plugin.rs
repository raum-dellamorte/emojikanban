use {
  crate::{
    ChatData, EmoteData, TwitchMgrCmd,
    config_kdl::{
      EkbConfigDirs, EkbConfigUpdate, EkbTwitchConfig,
      TWITCH_CALLBACK_URL,
      serve_oauth_receiver, validate_twitch_name,
    },
    effects::*,
    ekb_broadcast,
    font_studio::*,
    // plugin::{
    //   TwitchConnectionStatus::*, // TwitchOAuthRcvr::*,
    // },
    // start_twitch_monitor,
  },
  image::{
    AnimationDecoder, DynamicImage, ImageFormat,
    codecs::gif::GifDecoder,
  },
  obs_wrapper::{
    graphics::*,
    obs_string, 
    obs_sys::{
      OBS_SOURCE_CUSTOM_DRAW,
      obs_enter_graphics, 
      obs_leave_graphics,
      obs_source,
      obs_source_set_flags,
    },
    prelude::*, 
    properties::*, 
    source::*,
  },
  rand::prelude::*,
  std::{
    borrow::Cow,
    collections::VecDeque,
    // ops::Deref,
    sync::{
      Arc, Mutex,
    },
  },
  tokio::{
    runtime::Handle,
    sync::{
      broadcast,
      mpsc::{
        // UnboundedReceiver,
        UnboundedSender,
      },
    },
    // task::JoinHandle,
  }, 
};

pub struct EkbSettings {
  #[allow(dead_code)]
  source: WeakSourceRef,
  cmd_tx: UnboundedSender<TwitchMgrCmd>,
  config_draft: Arc<Mutex<EkbConfigUpdate>>,
}

impl EkbSettings {
  pub fn update_bot_account(&mut self, value: Cow<'_,str>) {
    let value = validate_twitch_name(value);
    if value.is_some() && let Ok(mut draft) = self.config_draft.lock() {
      draft.bot_account = value;
    } else {
      log::error!("Failed to lock config_draft.")
    }
  }
  pub fn update_channel(&mut self, value: Cow<'_,str>) {
    let value = validate_twitch_name(value);
    if value.is_some() && let Ok(mut draft) = self.config_draft.lock() {
      draft.channel = value; // fixme!
    } else {
      log::error!("Failed to lock config_draft.")
    }
  }
} // impl EkbSettings


impl Sourceable for EkbSettings {
  fn get_id() -> ObsString {
    obs_string!("emojikanban_settings")
  }
  fn get_type() -> SourceType {
    SourceType::Input
  }
  fn create(create: &mut CreatableSourceContext<Self>, mut source: SourceRef) -> Self {
    let settings = &mut create.settings;
    
    source.update_source_settings(settings);
    Self {
      source: source.downgrade(),
      cmd_tx: ekb_broadcast().cmd_tx.clone(),
      config_draft: Arc::new(Mutex::new(EkbConfigUpdate::default())),
    }
  }
}

impl GetNameSource for EkbSettings {
  fn get_name() -> ObsString {
    obs_string!("EmojiKanBan Settings")
  }
}

impl GetPropertiesSource for EkbSettings {
  fn get_properties(&mut self) -> Properties {
    let mut props = Properties::new();
    let cmd_tx = self.cmd_tx.clone();
    props.add_button_with_refresh(
      "twitch_authenticate".into(),
      "Request New Twitch OAuth Token And Connect".into(),
      true,
      move || {
        log::info!("EmojiKanBan attempting to (re)authenticate Twitch for access to chat. Server starting on http://localhost:3000/");
        std::thread::spawn({
          match open::that(TWITCH_CALLBACK_URL) {
            Ok(()) => {}
            Err(e) => { log::error!("Failed to open http://localhost:3000/ to acquire a Twitch OAuth Token. Please navigate manually to that address.\nError: {}", e) }
          }
          let cmd_tx = cmd_tx.clone();
          move || { match serve_oauth_receiver() {
            Ok(oauth) => {
              _ = cmd_tx.send(TwitchMgrCmd::UpdateConfig(
                EkbConfigUpdate { oauth: Some(oauth), ..Default::default() }
              )); }
            Err(e) => { log::error!("serve_oauth_receiver failure: {}", e); }
          }}
        });
      },
    );
    {
      let config_draft = self.config_draft.clone();
      let cmd_tx = self.cmd_tx.clone();
      props.add_button_with_refresh(
        "twitch_config_update".into(),
        "Apply Below Bot Account and Channel Values To Config".into(),
        true,
        move || {
          log::info!("EmojiKanBan updating config.kdl with new bot-account/channel values.");
          let update = match config_draft.lock() {
            Ok(mut draft) => std::mem::take(&mut *draft),
            Err(e) => {
              log::error!("Failed to lock config_draft: {}", e);
              return;
            }
          };
          if update.bot_account.is_none() && update.channel.is_none() && update.oauth.is_none() {
            log::info!("No valid config changes to apply.");
            return;
          }
          if let Err(e) = cmd_tx.send(TwitchMgrCmd::UpdateConfig(update)) {
            let TwitchMgrCmd::UpdateConfig(ref update) = e.0 else {
              return;
            };
            if let Ok(mut draft) = config_draft.lock() {
              *draft = update.clone();
            }
            log::error!("Failed to send config file update: {}", e)
          }
        },
      );
    };
    props
      .add(
        obs_string!("twitch_bot_account"),
        obs_string!("Twitch bot account"),
        TextProp::new(TextType::Default),
      )
      .add(
        obs_string!("twitch_channel"),
        obs_string!("Twitch channel"),
        TextProp::new(TextType::Default),
      );
    props
  }
}

impl UpdateSource for EkbSettings {
  fn update(&mut self, settings: &mut DataObj, _context: &mut GlobalContext) {
    let data = self;
    if let Some(bot_account) = settings.get(obs_string!("twitch_bot_account")) {
      data.update_bot_account(bot_account);
    }
    if let Some(channel) = settings.get(obs_string!("twitch_channel")) {
      data.update_channel(channel);
    }
  }
}

pub struct EmojiKanBan {
  source: WeakSourceRef,
  _runtime: Handle,
  chat_rx: broadcast::Receiver<Arc<ChatData>>,
  emote_queue: VecDeque<EmoteOBS>,
  emote_queue_max_length: u32,
  font_studio: FontStudio,
  rng: ThreadRng,
  screen_w: u32,
  screen_h: u32,
  screen_offset_x: u32,
  screen_offset_y: u32,
}

impl Sourceable for EmojiKanBan {
  fn get_id() -> ObsString {
    obs_string!("emojikanban")
  }
  fn get_type() -> SourceType {
    SourceType::Input
  }
  fn create(create: &mut CreatableSourceContext<Self>, mut source: SourceRef) -> Self {
    log::info!("Creating EmojiKanBan Context");
    let (_runtime, chat_rx) = {
      let broadcast = crate::ekb_broadcast();
      (broadcast.runtime.clone(), broadcast.chat_tx.subscribe())
    };
    let settings = &mut create.settings;
    let emote_queue_max_length = settings.get(obs_string!("emotes_max")).unwrap_or(200);
    let screen_w = settings.get(obs_string!("screen_width")).unwrap_or(1920);
    let screen_h = settings.get(obs_string!("screen_height")).unwrap_or(1080);
    let screen_offset_x = settings.get(obs_string!("offset_x")).unwrap_or(0);
    let screen_offset_y = settings.get(obs_string!("offset_y")).unwrap_or(0);
    
    let mut font_studio = FontStudio::new();
    font_studio.add_text_block(500, (50,50), (36.0,40.0), Some(15.0), "emojiKanBan Loaded");
    source.update_source_settings(settings);
    Self {
      source: source.downgrade(),
      _runtime,
      chat_rx,
      emote_queue: vec![].into(),
      emote_queue_max_length,
      font_studio,
      rng: rand::rng(),
      screen_w,
      screen_h,
      screen_offset_x,
      screen_offset_y,
    }
  }
}

const GRAVITY: f32 = 1800.;
const BOUNCE: f32 = 0.6;

impl GetNameSource for EmojiKanBan {
  fn get_name() -> ObsString {
    obs_string!("emojikanban")
  }
}

impl GetWidthSource for EmojiKanBan {
  fn get_width(&mut self) -> u32 {
    self.screen_w
  }
}

impl GetHeightSource for EmojiKanBan {
  fn get_height(&mut self) -> u32 {
    self.screen_h
  }
}

impl GetPropertiesSource for EmojiKanBan {
  fn get_properties(&mut self) -> Properties {
    let mut props = Properties::new();
    props
      .add(
        obs_string!("emotes_max"), 
        obs_string!("Cap the number of emotes to draw."), 
        NumberProp::new_int()
          .with_range(0..=1000)
          .with_slider(),
      )
      .add(
        obs_string!("screen_width"),
        obs_string!("Screen width"),
        NumberProp::new_int().with_range(1u32..=3840 * 3),
      )
      .add(
        obs_string!("screen_height"),
        obs_string!("Screen height"),
        NumberProp::new_int().with_range(1u32..=3840 * 3),
      )
      .add(
        obs_string!("offset_x"),
        obs_string!("Offset relative to the top left screen corner. X Offset:"),
        NumberProp::new_int().with_range(1u32..=3840 * 3),
      )
      .add(
        obs_string!("offset_y"),
        obs_string!("Offset relative to the top left screen corner. Y Offset:"),
        NumberProp::new_int().with_range(1u32..=3840 * 3),
      );
    props
  }
}

impl UpdateSource for EmojiKanBan {
  fn update(&mut self, settings: &mut DataObj, _context: &mut GlobalContext) {
    let data = self;
    if let Some(emotes_max) = settings.get(obs_string!("emotes_max")) {
      data.emote_queue_max_length = emotes_max;
    }
    if let Some(screen_width) = settings.get(obs_string!("screen_width")) {
      data.screen_w = screen_width;
    }
    if let Some(screen_height) = settings.get(obs_string!("screen_height")) {
      data.screen_h = screen_height;
    }
    if let Some(offset_x) = settings.get(obs_string!("offset_x")) {
      data.screen_offset_x = offset_x;
    }
    if let Some(offset_y) = settings.get(obs_string!("offset_y")) {
      data.screen_offset_y = offset_y;
    }
  }
}

impl VideoTickSource for EmojiKanBan {
  fn video_tick(&mut self, seconds: f32) {
    let data: &mut EmojiKanBan = self;
    let w = data.screen_w as f32;
    let h = data.screen_h as f32;
    // data.check_twitch_connection();
    loop { match data.chat_rx.try_recv() {
      Ok(ref chat_msg) => {
        data.font_studio.add_chat_msg(chat_msg.clone());
        for emote_data in chat_msg.emotes.iter() {
          let emote_data = emote_data.clone();
          if (data.emote_queue.len() as u32) < data.emote_queue_max_length {
            let mut emote: EmoteOBS = emote_data.into();
            if emote.tex_vec.is_empty() || emote.frame >= emote.tex_vec.len() {
              log::error!("tex_vec empty or current frame out of bounds: len: {} frame: {}", emote.tex_vec.len(), emote.frame);
              continue;
            }
            let (ew, eh) = (emote.tex_vec[emote.frame].width() as f32, emote.tex_vec[emote.frame].height() as f32);
            let picker = data.rng.random_range(1..=100);
            emote.effect = Some(match picker {
              1..=10 => {
                SlideUpEffect::init(
                  w,h,ew,eh,
                  &mut data.rng,
                )
              }
              11..=30 => {
                InchWormEffect::init(
                  w, h, ew, eh,
                  &mut data.rng
                )
              }
              31..=100 => {
                GravityEffect::init(
                  w,h,ew,eh,
                  GRAVITY, BOUNCE,
                  &mut data.rng,
                )
              }
              _ => { unreachable!() }
            });
            data.emote_queue.push_back(emote);
          }
        }
      }
      Err(broadcast::error::TryRecvError::Lagged(skipped)) => {
        log::warn!("Skipped {} stale chat messages", skipped);
      }
      Err(broadcast::error::TryRecvError::Empty) => break,
      Err(broadcast::error::TryRecvError::Closed) => {
        log::error!("EmojiKanBan chat broadcast closed");
        break;
      }
    }}
    // Animate emotes in queue
    for emote in data.emote_queue.iter_mut() {
      emote.update(seconds);
    }
    // Keep only the living
    data.emote_queue.retain(|emote| emote.is_alive() );
    data.font_studio.update(seconds);
  }
}

impl VideoRenderSource for EmojiKanBan {
  fn video_render(&mut self, _context: &mut GlobalContext, _render: &mut VideoRenderContext) {
    let data: &mut EmojiKanBan = self;
    unsafe {
      if let Some(source) = data.source.upgrade() {
        let source: *mut u8 = source.id() as *mut u8;
        obs_source_set_flags(source as *mut obs_source, OBS_SOURCE_CUSTOM_DRAW);
      }
      obs_enter_graphics();
      for emote in self.emote_queue.iter_mut() {
        if let Some(effect) = emote.effect.as_ref() {
          effect.draw(emote.current_frame());
        }
      }
      self.font_studio.draw();
      obs_leave_graphics();
    }
  }
}

pub struct EmoteOBS {
  pub name: String,
  tex_vec: Vec<GraphicsTexture>,
  delay: Vec<f32>,
  frame: usize,
  pub frame_time: f32,
  pub effect: Option<Box<dyn EmoteEffect>>,
}

impl EmoteOBS {
  pub fn is_alive(&self) -> bool {
    match self.effect.as_ref() {
      None => { false }
      Some(effect) => {
        effect.is_alive()
      }
    }
  }
  pub fn has_frames(&self) -> bool {
    !self.tex_vec.is_empty()
  }
  pub fn current_frame(&self) -> &GraphicsTexture {
    &self.tex_vec[self.frame]
  }
  pub fn current_delay(&self) -> f32 {
    self.delay[self.frame]
  }
  pub fn update(&mut self, seconds: f32) {
    if let Some(effect) = self.effect.as_mut() {
      effect.update(seconds);
    }
    if self.tex_vec.len() < 2 { return; }
    self.frame_time += seconds;
    if self.frame_time > self.delay[self.frame] {
      self.frame_time = 0.;
      self.frame = (self.frame + 1) % self.tex_vec.len();
    }
  }
}

impl From<EmoteData> for EmoteOBS { // This approach is fun but doesn't allow for error handling outside of log::error!()
  fn from(value: EmoteData) -> Self {
    let mut tex_vec: Vec<GraphicsTexture> = vec![];
    let mut delay: Vec<f32> = vec![];
    match image::guess_format(&value.img) {
      Err(e) => { log::error!("Failed to guess_format of image data: {}", e) }
      Ok(ImageFormat::Gif) => {
        let cursor = std::io::Cursor::new(&value.img);
        let gifdec_result = GifDecoder::new(cursor);
        if let Ok(gif) = gifdec_result {
          let frames = gif.into_frames();
          for frame_result in frames.into_iter() {
            let mut width = 0;
            let mut height = 0;
            let mut linesize = 0;
            match frame_result {
              Err(e) => { log::error!("Failed to decode GIF from image data: {}", e); }
              Ok(frame) => {
                let (ms,_) = frame.delay().numer_denom_ms();
                let d = (ms as f32) / 1000.;
                let img = DynamicImage::ImageRgba8(frame.into_buffer());
                if width == 0 {
                  (width, height) = (img.width(), img.height());
                  linesize = width * 4;
                }
                let mut texture = GraphicsTexture::new(
                  width, height, 
                  GraphicsColorFormat::RGBA,
                );
                let pixels = img.into_rgba8().into_raw();
                texture.set_image(&pixels, linesize, false);
                tex_vec.push(texture);
                delay.push(d);
              }
            }
          }
        }
      }
      Ok(_) => {
        if let Ok(img) = image::load_from_memory(&value.img) {
          let mut texture = GraphicsTexture::new(
            img.width(), img.height(), 
            GraphicsColorFormat::RGBA,
          );
          let linesize = img.width() * 4; // pixels wide * 4 bytes per pixel for RGBA
          let pixels = img.into_rgba8().into_raw();
          texture.set_image(&pixels, linesize, false);
          tex_vec.push(texture);
        };
      }
    }
    Self{
      name: value.name,
      tex_vec,
      delay,
      frame: 0,
      frame_time: 0.,
      effect: None,
    }
  }
}

#[derive(Debug)]
pub enum TwitchOAuthRcvr {
  OAuthToken(String),
  NewConfigData((EkbConfigDirs, EkbTwitchConfig)),
  RcvrError(anyhow::Error),
}

// #[derive(Debug)]
// enum TwitchConnectionStatus {
//   InitConnection,
//   AwaitingConfig,
//   Connected,
// }
