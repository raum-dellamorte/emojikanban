// #![allow(dead_code,unused)]
use {
  // crate::{
  //   config_kdl::*,
  //   plugin::{
  //     TwitchOAuthRcvr::*,
  //     *,
  //   },
  // },
  // anyhow::{
  //   Result,
  //   anyhow,
  // },
  image::{
    // AnimationDecoder, DynamicImage, ImageFormat,
    Rgba, RgbaImage,
    // codecs::gif::GifDecoder,
  },
  cosmic_text::{
    Attrs, AttrsOwned, Color, FontSystem, SwashCache, Buffer, Metrics, Shaping,
  },
  // futures::StreamExt,
  obs_wrapper::graphics::*,
  std::{
    collections::VecDeque,
    // sync::Arc,
  },
  // tokio::sync::mpsc::UnboundedSender,
};

pub struct FontStudio {
  font_system: FontSystem,
  swash_cache: SwashCache,
  buffer: Buffer,
  attrs: AttrsOwned,
  pub text_blocks: VecDeque<TextBlock>,
  #[allow(dead_code)]
  screen_w: f32,
  #[allow(dead_code)]
  screen_h: f32,
}

impl FontStudio {
  pub fn new() -> Self {
    let mut font_system = FontSystem::new();
    let swash_cache = SwashCache::new();
    let metrics = Metrics::new(20.0, 24.0);
    let buffer = Buffer::new(&mut font_system, metrics);
    let attrs = Attrs::new();
    let attrs = AttrsOwned::new(&attrs);
    Self {
      font_system,
      swash_cache,
      buffer,
      attrs,
      text_blocks: VecDeque::new(),
      screen_w: 1920.0, // probably fixme
      screen_h: 1080.0,
    }
  }
  pub fn update_dimensions(&mut self, w: f32, h: f32) {
    (self.screen_w, self.screen_h) = (w, h);
  }
  pub fn add_text_block(&mut self, image_width: u32, offset: (i32,i32), metrics: (f32,f32), life: Option<f32>, txt: &str) {
    let (x_offset, y_offset) = offset;
    let mut buffer = self.buffer.borrow_with(&mut self.font_system);
    let text_color = Color::rgb(0xFF, 0xFF, 0xFF);
    let img_w = image_width.max(MIN_WIDTH);
    let inner_w = img_w - (2 * PADDING);
    buffer.set_size(Some(inner_w as f32), None);
    buffer.set_metrics(Metrics::new(metrics.0, metrics.1));
    buffer.set_text(txt, &self.attrs.as_attrs(), Shaping::Advanced, None);
    buffer.shape_until_scroll(false);
    let img_h = buffer.layout_runs().map(|run| run.line_top + run.line_height)
        .fold(0.0f32, f32::max).ceil() as u32 + (2 * PADDING);
    let mut img = RgbaImage::from_pixel(img_w, img_h, Rgba([0,0,0,0]));
    buffer.draw(&mut self.swash_cache, text_color, |x,y,_w,_h,color| {
      if color.a() == 0 { return; }
      let img_x = x + PADDING as i32;
      let img_y = y + PADDING as i32;
      if img_x < 0 || img_y < 0 || img_x >= img_w as i32 || img_y >= img_h as i32 {
        return;
      }
      let pxl = img.get_pixel_mut(img_x as u32, img_y as u32);
      // let src_a = u16::from(color.a());
      // let dst_a = src_a + u16::from(pxl[3]) * (255 - src_a) / 255; // If we want to draw text on and existing image
      *pxl = Rgba([
        color.r(), color.g(), color.b(), color.a(),
      ]);
    });
    let mut tex = GraphicsTexture::new(
      img.width(), img.height(), 
      GraphicsColorFormat::RGBA,
    );
    let linesize = img.width() * 4; // pixels wide * 4 bytes per pixel for RGBA
    let pixels = img.into_raw();
    tex.set_image(&pixels, linesize, false);
    let tblk = TextBlock { tex, life, x_offset, y_offset, };
    self.text_blocks.push_back(tblk);
  }
  pub fn draw(&self) {
    for tblk in self.text_blocks.iter() {
      tblk.draw();
    }
  }
}

pub struct TextBlock {
  tex: GraphicsTexture,
  life: Option<f32>,
  x_offset: i32,
  y_offset: i32,
}

impl TextBlock {
  pub fn draw(&self) {
    self.tex.draw(self.x_offset, self.y_offset, 0, 0, false);
  }
  pub fn is_alive(&self) -> bool {
    self.life.is_none() || (self.life.is_some() && self.life.unwrap() > 0.0)
  }
  pub fn update(&mut self, seconds: f32) {
    if let Some(life) = self.life.as_mut() {
      *life -= seconds;
    }
  } 
}

const PADDING: u32 = 10;
const MIN_WIDTH: u32 = 40;
pub const LOREM_IPSUM: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, \
sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim \
veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. \
Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat \
nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia \
deserunt mollit anim id est laborum.";

