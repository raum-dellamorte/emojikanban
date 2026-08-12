use {
  crate::{
    config_kdl::{
      EkbConfigDirs, EkbTwitchConfig, 
    },
    effects::*,
    EmoteData,
    EmoteComEnum,
  },
  image::{
    AnimationDecoder,
    DynamicImage,
    ImageFormat,
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
  std::collections::VecDeque,
  tokio::{
    task::JoinHandle,
    runtime::Runtime,
    sync::mpsc::{
      UnboundedReceiver,UnboundedSender,
    },
  }, 
};

#[allow(dead_code)]
enum TwitchConnectionStatus {
  AuthenticateTwitch,
  Connected(
    EkbConfigDirs, EkbTwitchConfig,
  ),
  FailedToConnect(anyhow::Error),
}

// enum TwitchStatus {
//   Connected,
//   Disconnected,
// }

pub struct EmojiKanBan {
  id: usize,
  #[allow(dead_code)]
  runtime: Option<Runtime>,
  cmd_tx: Option<UnboundedSender<TwitchConnectionStatus>>,
  cmd_rx: Option<UnboundedReceiver<TwitchConnectionStatus>>,
  #[allow(dead_code)]
  twitch_monitor: Option<JoinHandle<()>>,
  // twitch_status: TwitchStatus,
  emote_rx: Option<UnboundedReceiver<EmoteComEnum>>, // EmoteData -> anyhow::Result<EmoteData, String> to return error to try to reconnect to Twitch
  emote_queue: VecDeque<EmoteOBS>,
  emote_queue_max_length: u32,
  rng: ThreadRng,
  screen_w: u32,
  screen_h: u32,
  screen_offset_x: u32,
  screen_offset_y: u32,
}

impl Drop for EmojiKanBan {
  fn drop(&mut self) {
    if let Some(mut rx) = self.emote_rx.take() {
      rx.close();
      while rx.blocking_recv().is_some() {}
    }
    if let Some(runtime) = self.runtime.take() {
      runtime.shutdown_timeout(std::time::Duration::from_nanos(500));
      // runtime.shutdown_background();
    }
    // unsafe {
    //   let source: *mut u8 = self.id as *mut u8;
    //   let source = source as *mut obs_source;
    //   obs_wrapper::obs_sys::obs_source_remove(source);
    //   // obs_wrapper::obs_sys::obs_source_release(source);
    // }
  }
}

impl Sourceable for EmojiKanBan {
  fn get_id() -> ObsString {
    obs_string!("emojikanban")
  }
  fn get_type() -> SourceType {
    SourceType::Input
  }
  fn create(create: &mut CreatableSourceContext<Self>, mut source: SourceRef) -> Self {
    let settings = &mut create.settings;
    let emote_queue_max_length = settings.get(obs_string!("emotes_max")).unwrap_or(200);
    let screen_w = settings.get(obs_string!("screen_width")).unwrap_or(1920);
    let screen_h = settings.get(obs_string!("screen_height")).unwrap_or(1080);
    let screen_offset_x = settings.get(obs_string!("offset_x")).unwrap_or(0);
    let screen_offset_y = settings.get(obs_string!("offset_y")).unwrap_or(0);
    
    source.update_source_settings(settings);
    
    let mut ekb = Self {
      id: source.id(),
      runtime: None,
      cmd_tx: None,
      cmd_rx: None,
      twitch_monitor: None,
      emote_rx: None,
      emote_queue: vec![].into(),
      emote_queue_max_length,
      rng: rand::rng(),
      screen_w,
      screen_h,
      screen_offset_x,
      screen_offset_y,
    };
    ekb.connect_twitch();
    ekb
  }
}

impl EmojiKanBan {
  pub fn connect_twitch(&mut self) {
    if let Some(runtime) = self.runtime.as_mut() {
      let ekbc: anyhow::Result<(EkbConfigDirs, EkbTwitchConfig),String> = runtime.block_on(async {
        crate::get_or_create_config_emojikanban(None).await
      });
      if let Ok((ekb_config_dirs, conf)) = ekbc {
        if let Some(mut rx) = self.cmd_rx.take() {
          rx.close();
          _ = self.cmd_tx.take();
          while rx.blocking_recv().is_some() {}
        }
        if let Some(mut rx) = self.emote_rx.take() {
          rx.close();
          while rx.blocking_recv().is_some() {}
        }
        let (cmd_tx, cmd_rx) = tokio::sync::mpsc::unbounded_channel();
        let (emote_tx, emote_rx) = tokio::sync::mpsc::unbounded_channel();
        runtime.spawn(async move {
          crate::start_twitch_monitor(ekb_config_dirs, conf, emote_tx).await;
        });
        self.cmd_tx = Some(cmd_tx);
        self.cmd_rx = Some(cmd_rx);
        self.emote_rx = Some(emote_rx);
      }
    } else {
      self.runtime = Some(tokio::runtime::Runtime::new().unwrap());
      self.connect_twitch();
    }
  }
} // impl EmojiKanBan

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
      .add_button(
        "twitch_authenticate".into(),
        "Connect Twitch".into(),
        move || {
          // something
        },
      )
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
    if let Some(rx) = data.emote_rx.as_mut() {
      while let Ok(emote_data) = rx.try_recv() {
        if (data.emote_queue.len() as u32) < data.emote_queue_max_length {
          let emote_data = match emote_data {
            EmoteComEnum::Data(emote_data) => { emote_data }
            EmoteComEnum::TwitchConnectionFailure(_e) => {
              // set status to not connected or something
              return;
            }
            EmoteComEnum::SqliteConnectionFailure(_e) => {
              // like, let somebody know, you know?
              return;
            }
          };
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
        } else {
          let _ = emote_data;
        }
      }
    }
    // Animate emotes in queue
    for emote in data.emote_queue.iter_mut() {
      emote.update(seconds);
    }
    // Keep only the living
    data.emote_queue.retain(|emote| emote.is_alive() );
  }
}

impl VideoRenderSource for EmojiKanBan {
  fn video_render(&mut self, _context: &mut GlobalContext, _render: &mut VideoRenderContext) {
    let data: &mut EmojiKanBan = self;
    unsafe {
      {
        let source: *mut u8 = data.id as *mut u8;
        obs_source_set_flags(source as *mut obs_source, OBS_SOURCE_CUSTOM_DRAW);
      }
      obs_enter_graphics();
      for emote in self.emote_queue.iter_mut() {
        if let Some(effect) = emote.effect.as_ref() {
          effect.draw(emote.current_frame());
        }
      }
      obs_leave_graphics();
    }
  }
}

pub struct EmoteOBS {
  pub name: String,
  tex_vec: Vec<GraphicsTexture>, // Make this a Vec<GraphicsTexture> to support animation
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
