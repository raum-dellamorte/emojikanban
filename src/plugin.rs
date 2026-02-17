use {
  crate::effects::*,
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
    runtime::Runtime,
    sync::mpsc::UnboundedReceiver,
  }, 
};

pub struct EmojiKanBan {
  source: SourceContext,
  #[allow(dead_code)]
  runtime: Runtime,
  rx: UnboundedReceiver<EmoteData>, 
  emote_queue: VecDeque<EmoteOBS>,
  emote_queue_max_length: u32,
  rng: ThreadRng,
  padding: f64,
  opacity: u32,
  screen_w: u32,
  screen_h: u32,
  screen_x: u32,
  screen_y: u32,
}

impl Sourceable for EmojiKanBan {
  fn get_id() -> ObsString {
    obs_string!("emojikanban")
  }
  fn get_type() -> SourceType {
    SourceType::INPUT
  }
  fn create(create: &mut CreatableSourceContext<Self>, mut source: SourceContext) -> Self {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    runtime.spawn(async move {
      if let Err(e) = crate::run(tx).await {
        log::error!("Twitch monitor died: {}", e);
      }
    });
    
    let settings = &mut create.settings;
    let screen_w = settings.get(obs_string!("screen_width")).unwrap_or(1920);
    let screen_h = settings.get(obs_string!("screen_height")).unwrap_or(1080);
    let screen_x = settings.get(obs_string!("screen_x")).unwrap_or(0);
    let screen_y = settings.get(obs_string!("screen_y")).unwrap_or(0);
    let emote_queue_max_length = settings.get(obs_string!("emotes_max")).unwrap_or(200);
    
    source.update_source_settings(settings);
    
    Self {
      source,
      runtime,
      rx,
      emote_queue: vec![].into(),
      emote_queue_max_length,
      rng: rand::rng(),
      padding: 0.1,
      opacity: 255,
      screen_w,
      screen_h,
      screen_x,
      screen_y,
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
        obs_string!("opacity"), 
        obs_string!("Change the opacity of the emotes."), 
        NumberProp::new_int()
          .with_range(0..=255)
          .with_slider(),
      )
      .add(
        obs_string!("emotes_max"), 
        obs_string!("Cap the number of emotes to draw."), 
        NumberProp::new_int()
          .with_range(0..=1000)
          .with_slider(),
      )
      .add(
        obs_string!("screen_x"),
        obs_string!("Offset relative to top left screen - x"),
        NumberProp::new_int().with_range(1u32..=3840 * 3),
      )
      .add(
        obs_string!("screen_y"),
        obs_string!("Offset relative to top left screen - y"),
        NumberProp::new_int().with_range(1u32..=3840 * 3),
      )
      .add(
        obs_string!("padding"),
        obs_string!("Padding around each window"),
        NumberProp::new_float(0.001)
          .with_range(..=0.5)
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
      );
    props
  }
}

impl UpdateSource for EmojiKanBan {
  fn update(&mut self, settings: &mut DataObj, _context: &mut GlobalContext) {
    let data = self;
    if let Some(opacity) = settings.get(obs_string!("opacity")) {
      data.opacity = opacity;
    }
    if let Some(emotes_max) = settings.get(obs_string!("emotes_max")) {
      data.emote_queue_max_length = emotes_max;
    }
    if let Some(screen_width) = settings.get(obs_string!("screen_width")) {
      data.screen_w = screen_width;
    }
    if let Some(screen_height) = settings.get(obs_string!("screen_height")) {
      data.screen_h = screen_height;
    }
    if let Some(screen_x) = settings.get(obs_string!("screen_x")) {
      data.screen_x = screen_x;
    }
    if let Some(screen_y) = settings.get(obs_string!("screen_y")) {
      data.screen_y = screen_y;
    }
    if let Some(padding) = settings.get(obs_string!("padding")) {
      data.padding = padding;
    }
  }
}

impl VideoTickSource for EmojiKanBan {
  fn video_tick(&mut self, seconds: f32) {
    let data: &mut EmojiKanBan = self;
    let w = data.screen_w as f32;
    let h = data.screen_h as f32;
    while let Ok(emote_data) = data.rx.try_recv() {
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
      } else {
        let _ = emote_data;
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
        let id: usize = { data.source.id().to_owned() };
        let source: *mut u8 = id as *mut u8;
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

#[derive(Clone)]
pub struct EmoteData {
  pub id: String,
  pub name: String,
  pub img: Vec<u8>,
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
