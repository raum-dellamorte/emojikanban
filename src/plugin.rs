use {
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
    
    source.update_source_settings(settings);
    
    Self {
      source,
      runtime,
      rx,
      emote_queue: vec![].into(),
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
      )
      .add(
        obs_string!("animation_time"),
        obs_string!("Animation Time (s)"),
        NumberProp::new_float(0.001).with_range(0.3..=10.),
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
    while let Ok(emote_data) = data.rx.try_recv() {
      if data.emote_queue.len() < 100 {
        let mut emote_obs: EmoteOBS = emote_data.into();
        let x = data.rng.random_range(0.1..0.9) as f32;
        let vx = data.rng.random_range(-0.15..0.15);
        emote_obs.life_total = data.rng.random_range(2.0..5.0);
        emote_obs.pos.set(x, 0., 0.);
        emote_obs.vel.set(vx, 0., 0.);
        data.emote_queue.push_back(emote_obs);
      } else {
        let _ = emote_data;
      }
    }
    // Animate emotes in queue
    let w = data.screen_w as f32;
    let h = data.screen_h as f32;
    let g = GRAVITY / h;
    for emote in data.emote_queue.iter_mut() {
      emote.life_lived += seconds;
      let mut x = emote.pos.x();
      let mut y = emote.pos.y();
      let ew = emote.tex.width() as f32 / w;
      let eh = emote.tex.height() as f32 / h;
      // Update velocity
      let mut vy = emote.vel.y();
      vy += g * seconds;
      let vx = emote.vel.x();
      emote.vel.set(vx, vy, 0.);
      // Apply velocity
      x += vx * seconds;
      y += vy * seconds;
      emote.pos.set(x, y, 0.);
      
      // Bounce
      let floor: f32 = 1.0 - eh;
      if y > floor {
        emote.pos.set(x, floor, 0.);
        emote.vel.set(vx, -vy * BOUNCE, 0.);
      }
      if x < 0. || x >= 1.0 - ew {
        emote.vel.set(-vx, vy, 0.);
      }
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
        let x = (emote.pos.x() * 1920.) as i32;
        let y = (emote.pos.y() * 1080.) as i32;
        emote.tex.draw(x, y, 0, 0, false);
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
  pub tex: GraphicsTexture,
  pub life_total: f32,
  pub life_lived: f32,
  pub pos: Vec3,
  pub vel: Vec3,
  pub scale: Vec3,
}

impl EmoteOBS {
  pub fn is_alive(&self) -> bool { self.life_lived < self.life_total }
}

impl From<EmoteData> for EmoteOBS {
  fn from(value: EmoteData) -> Self {
    let tex = if let Ok(img) = image::load_from_memory(&value.img) {
      let mut tex = GraphicsTexture::new(
        img.width(), img.height(), 
        GraphicsColorFormat::RGBA,
      );
      let linesize = img.width() * 4; // pixels wide * 4 bytes per pixel for RGBA
      let pixels = img.into_rgba8().into_raw();
      tex.set_image(&pixels, linesize, false);
      tex
    } else { unreachable!() };
    Self{
      name: value.name,
      tex,
      life_total: 0.,
      life_lived: 0.,
      pos: Vec3::default(),
      vel: Vec3::default(),
      scale: Vec3::default(),
    }
  }
}
