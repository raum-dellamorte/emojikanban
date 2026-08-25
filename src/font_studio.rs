// #![allow(dead_code,unused)]
use {
  crate::ChatData,
  cosmic_text::{
    Attrs, AttrsOwned, Buffer, Color, FeatureTag, FontFeatures, FontSystem, Metrics, Shaping, SwashCache, Weight,
  },
  image::{
    // AnimationDecoder, DynamicImage, ImageFormat,
    Rgba, RgbaImage,
    // codecs::gif::GifDecoder,
  },
  obs_wrapper::graphics::*,
  std::collections::VecDeque,
};

pub struct FontStudio {
  font_system: FontSystem,
  swash_cache: SwashCache,
  buffer: Buffer,
  attrs: AttrsOwned,
  pub text_blocks: VecDeque<TextBlock>,
  pub chat_blocks: VecDeque<ChatMsgBlock>,
  screen_w: f32,
  screen_h: f32,
  user_metrics: (f32,f32),
  chat_bg_tex: Option<GraphicsTexture>,
  chat_life: f32,
  chat_offset: (i32,i32),
  chat_metrics: (f32,f32),
  chat_margin: i32,
  chat_width: u32,
}

impl FontStudio {
  pub fn new() -> Self {
    let mut font_system = FontSystem::new();
    let swash_cache = SwashCache::new();
    let buffer = Buffer::new(&mut font_system, Metrics::new(20.0, 24.0)); // The metrics here don't matter, we reset it when text is added
    let attrs = Attrs::new();
    let attrs = AttrsOwned::new(&attrs);
    let cbg_img = create_chat_bg(300,700,20.0,[10,50,10,179]);
    let chat_bg_tex = Some(gen_rgba_tex(cbg_img));
    Self {
      font_system,
      swash_cache,
      buffer,
      attrs,
      text_blocks: VecDeque::new(),
      chat_blocks: VecDeque::new(),
      screen_w: 1920.0, // probably fixme
      screen_h: 1080.0,
      user_metrics: (26.0, 30.0),
      chat_bg_tex,
      chat_life: 120.0,
      chat_offset: (1620,0),
      chat_metrics: (30.0,34.0),
      chat_margin: 10,
      chat_width: 300,
    }
  }
  pub fn update_dimensions(&mut self, w: f32, h: f32) {
    (self.screen_w, self.screen_h) = (w, h);
  }
  pub fn update(&mut self, seconds: f32) {
    self.text_blocks.retain(|tblk| tblk.is_alive() );
    self.chat_blocks.retain(|tblk| tblk.is_alive() );
    for tblk in self.text_blocks.iter_mut() {
      tblk.update(seconds);
    }
    let mut current_y = self.chat_offset.1 + self.chat_margin;
    for cblk in self.chat_blocks.iter_mut() {
      cblk.set_y(current_y);
      cblk.update(seconds);
      current_y += cblk.height() as i32;
    }
  }
  pub fn add_text_block(&mut self, image_width: u32, offset: (i32,i32), metrics: (f32,f32), life: Option<f32>, txt: &str) {
    let mut buffer = self.buffer.borrow_with(&mut self.font_system);
    let (x_offset, y_offset) = offset;
    let text_color = Color::rgb(0xFF, 0xFF, 0xFF);
    let img_w = image_width.max(MIN_WIDTH);
    let inner_w = img_w - (2 * self.chat_margin as u32);
    buffer.set_size(Some(inner_w as f32), None);
    buffer.set_metrics(Metrics::new(metrics.0, metrics.1));
    buffer.set_text(txt, &self.attrs.as_attrs(), Shaping::Advanced, None);
    buffer.shape_until_scroll(false);
    let img_h = buffer.layout_runs().map(|run| run.line_top + run.line_height)
      .fold(0.0f32, f32::max).ceil() as u32 + (2 * self.chat_margin as u32);
    let mut img = RgbaImage::from_pixel(img_w, img_h, Rgba([0,0,0,0]));
    buffer.draw(&mut self.swash_cache, text_color, draw_buffer(&mut img, self.chat_margin));
    let tex = gen_rgba_tex(img);
    let tblk = TextBlock { tex, life, x_offset, y_offset, };
    self.text_blocks.push_back(tblk);
  }
  pub fn add_chat_msg(&mut self, msg: ChatData) {
    let mut buffer = self.buffer.borrow_with(&mut self.font_system);
    let (x_offset, y_offset) = self.chat_offset;
    let (font_size, line_height) = self.chat_metrics;
    let img_w = self.chat_width.max(MIN_WIDTH);
    let inner_w = img_w - (2 * self.chat_margin) as u32;
    let user: &str = &msg.user;
    let user_attrs = Attrs::new()
      .color(msg.uname_color.unwrap_or(Color::rgb(0xC8, 0x64, 0xC8)))
      .font_features(FontFeatures::new().enable(FeatureTag::SMALL_CAPS).to_owned())
      .metrics(Metrics::new(self.user_metrics.0, self.user_metrics.1))
      .weight(Weight::BOLD);
    let msg_ptr_attrs = Attrs::new()
      .color(Color::rgb(0x36, 0x87, 0x77))
      .metrics(Metrics::new(self.chat_metrics.0, self.chat_metrics.1))
      .weight(Weight::BOLD);
    buffer.set_size(Some(inner_w as f32), None);
    buffer.set_metrics(Metrics::new(font_size, line_height));
    buffer.set_rich_text(
      [
        (user, user_attrs.clone()),
        (":\n", user_attrs),
        (MSG_PTR, msg_ptr_attrs),
      ],
      &Attrs::new(),
      Shaping::Advanced,
      None,
    );
    buffer.shape_until_scroll(false);
    let img_h = buffer.layout_runs().map(|run| run.line_top + run.line_height)
      .fold(0.0f32, f32::max).ceil() as u32 + (2 * self.chat_margin as u32);
    let msg_indent: i32 = buffer.layout_runs().last().map(|run| run.line_w.ceil() as i32 )
        .unwrap_or(0) + self.chat_margin;
    let mut usr_img = RgbaImage::from_pixel(img_w, img_h, Rgba([0,0,0,0]));
    let text_color = Color::rgb(0xFF, 0xFF, 0xFF);
    buffer.draw(&mut self.swash_cache, text_color, draw_buffer(&mut usr_img, self.chat_margin));
    let usr_tex = gen_rgba_tex(usr_img);
    // Generate chat message image
    let msg_string: String = if msg.emotes.len() == 0 { msg.msg.to_owned() } else {
      let mut filtered = String::with_capacity(msg.msg.len());
      let mut i = 0;
      for emote in msg.emotes.iter() {
        if emote.loc.0 >= i {
          filtered.push_str( &msg.msg[i..emote.loc.0] );
        }
        filtered.push_str( EMOTE_PLACEHOLDER );
        if emote.loc.1 > i { i = emote.loc.1; }
      }
      if i < msg.msg.len() { filtered.push_str( &msg.msg[i..] ); }
      filtered
    };
    let msg_txt: &str = &msg_string;
    let msg_attrs = Attrs::new().color(Color::rgb(0xC8, 0xC8, 0xC8));
    let emote_attrs = Attrs::new().color(Color::rgba(0, 0, 0, 0));
    buffer.set_size(Some((inner_w as i32 - msg_indent) as f32), None);
    let img_h = if msg.emotes.len() == 0 {
      buffer.set_text(msg_txt, &msg_attrs, Shaping::Advanced, None);
      buffer.shape_until_scroll(false);
      buffer.layout_runs().map(|run| run.line_top + run.line_height)
        .fold(0.0f32, f32::max).ceil() as u32 + (2 * self.chat_margin as u32)
    } else {
      let mut parts = msg_string.split(EMOTE_PLACEHOLDER).peekable();
      let mut spans = Vec::new();
      let mut emote_index = 0;
      while let Some(text) = parts.next() {
        if !text.is_empty() {
          spans.push((text, msg_attrs.clone()));
        }
        if parts.peek().is_some() {
          spans.push((EMOTE_PLACEHOLDER, emote_attrs.clone().metadata(emote_index + 1)));
          emote_index += 1;
        }
      }
      buffer.set_rich_text(
        spans,
        &Attrs::new(),
        Shaping::Advanced,
        None,
      );
      buffer.shape_until_scroll(false);
      // let mut out = 0.0;
      // for run in buffer.layout_runs() {
      //   // for glyph in run.glyphs {
      //   //   if glyph.metadata != 0 {
      //   //     let i = glyph.metadata - 1;
      //   //     msg.emotes[i].img.clone();
      //   //   }
      //   // }
      //   out = (run.line_top + run.line_height).max(out);
      // }
      // out.ceil() as u32 + (2 * PADDING)
      buffer.layout_runs().map(|run| run.line_top + run.line_height)
        .fold(0.0f32, f32::max).ceil() as u32 + (2 * self.chat_margin as u32)
    };
    let mut msg_img = RgbaImage::from_pixel(inner_w, img_h, Rgba([0,0,0,0]));
    let text_color = Color::rgb(0xFF, 0xFF, 0xFF);
    buffer.draw(&mut self.swash_cache, text_color, draw_buffer(&mut msg_img, self.chat_margin));
    let msg_img = add_text_outline(&msg_img, 2, Rgba([170,0,0,255]));
    let msg_tex = gen_rgba_tex(msg_img);
    let cblk = ChatMsgBlock {
      usr_tex, msg_tex, chat_data: msg, life: Some(self.chat_life),
      x_offset, y_offset,
      msg_y_offset: self.user_metrics.1 as i32,
      msg_indent, 
    };
    self.chat_blocks.push_back(cblk);
  }
  pub fn draw(&self) {
    if let Some(bg) = self.chat_bg_tex.as_ref() {
      bg.draw(self.chat_offset.0, self.chat_offset.1, 0, 0, false);
    }
    for tblk in self.text_blocks.iter() {
      tblk.draw();
    }
    for cblk in self.chat_blocks.iter() {
      cblk.draw();
    }
  }
}

pub trait FontStudioTextBlock {
  fn draw(&self);
  fn is_alive(&self) -> bool;
  fn height(&self) -> u32;
  fn update(&mut self, seconds: f32);
  fn set_x(&mut self, val: i32);
  fn set_y(&mut self, val: i32);
}

pub struct ChatMsgBlock {
  usr_tex: GraphicsTexture,
  msg_tex: GraphicsTexture,
  pub chat_data: ChatData,
  life: Option<f32>,
  pub x_offset: i32,
  pub y_offset: i32,
  msg_y_offset: i32,
  msg_indent: i32,
}

impl FontStudioTextBlock for ChatMsgBlock {
  fn draw(&self) {
    self.usr_tex.draw(self.x_offset, self.y_offset, 0, 0, false);
    self.msg_tex.draw(self.x_offset + self.msg_indent, self.y_offset + self.msg_y_offset, 0, 0, false);
  }
  fn is_alive(&self) -> bool {
    self.life.is_none_or(|life| life > 0.0)
  }
  fn height(&self) -> u32 {
    self.msg_y_offset as u32 + self.msg_tex.height()
  }
  fn update(&mut self, seconds: f32) {
    if let Some(life) = self.life.as_mut() {
      *life -= seconds;
    }
  }
  fn set_x(&mut self, val: i32) {
    self.x_offset = val;
  }
  fn set_y(&mut self, val: i32) {
    self.y_offset = val;
  }
}

pub struct TextBlock {
  tex: GraphicsTexture,
  life: Option<f32>,
  x_offset: i32,
  y_offset: i32,
}

impl FontStudioTextBlock for TextBlock {
  fn draw(&self) {
    self.tex.draw(self.x_offset, self.y_offset, 0, 0, false);
  }
  fn is_alive(&self) -> bool {
    self.life.is_none_or(|life| life > 0.0)
  }
  fn height(&self) -> u32 {
    self.tex.height()
  }
  fn update(&mut self, seconds: f32) {
    if let Some(life) = self.life.as_mut() {
      *life -= seconds;
    }
  } 
  fn set_x(&mut self, val: i32) {
    self.x_offset = val;
  }
  fn set_y(&mut self, val: i32) {
    self.y_offset = val;
  }
}

fn gen_rgba_tex(img: RgbaImage) -> GraphicsTexture {
  let mut tex = GraphicsTexture::new(
    img.width(), img.height(), 
    GraphicsColorFormat::RGBA,
  );
  let linesize = img.width() * 4; // pixels wide * 4 bytes per pixel for RGBA
  let pixels = img.into_raw();
  tex.set_image(&pixels, linesize, false);
  tex
}

fn draw_buffer(img: &mut RgbaImage, padding: i32) -> impl FnMut(i32, i32, u32, u32, Color) {
  move |x,y,_w,_h,color| {
    if color.a() == 0 { return; }
    let img_x = x + padding;
    let img_y = y + padding;
    if img_x < 0 || img_y < 0 || img_x >= img.width() as i32 || img_y >= img.height() as i32 {
      return;
    }
    let pxl = img.get_pixel_mut(img_x as u32, img_y as u32);
    // let src_a = u16::from(color.a());
    // let dst_a = src_a + u16::from(pxl[3]) * (255 - src_a) / 255; // If we want to draw text on and existing image
    *pxl = Rgba([ color.r(), color.g(), color.b(), color.a(), ]);
  }
}

fn create_chat_bg(width: u32, height: u32, rounding: f32, bg: [u8;4]) -> RgbaImage {
  let mut image = RgbaImage::from_pixel(width, height, Rgba([0, 0, 0, 0]));
  let center_x = width as f32 / 2.0;
  let center_y = height as f32 / 2.0;
  let inner_half_width = center_x - rounding;
  let inner_half_height = center_y - rounding;
  for y in 0..height {
    for x in 0..width {
      // Measure from the center of the pixel.
      let pixel_x = x as f32 + 0.5;
      let pixel_y = y as f32 + 0.5;
      // Signed distance from a rounded rectangle.
      let distance_x = (pixel_x - center_x).abs() - inner_half_width;
      let distance_y = (pixel_y - center_y).abs() - inner_half_height;
      let outside_x = distance_x.max(0.0);
      let outside_y = distance_y.max(0.0);
      let outside_distance = outside_x.hypot(outside_y);
      let inside_distance = distance_x.max(distance_y).min(0.0);
      let signed_distance = outside_distance + inside_distance - rounding;
      // A one-pixel antialiasing transition around the edge.
      let coverage = (0.5 - signed_distance).clamp(0.0, 1.0);
      if coverage == 0.0 { continue; }
      let alpha = (bg[3] as f32 * coverage).round() as u8;
      image.put_pixel(
        x,
        y,
        Rgba([ bg[0], bg[1], bg[2], alpha, ]),
      );
    }
  }
  image
}

fn add_text_outline(
  text: &RgbaImage,
  radius: u32,
  outline_color: Rgba<u8>,
) -> RgbaImage {
  let mut outlined =
      RgbaImage::from_pixel(text.width(), text.height(), Rgba([0, 0, 0, 0]));
  let radius = radius as i32;
  let radius_squared = radius * radius;
  // Dilate the text's alpha mask.
  for y in 0..text.height() as i32 {
    for x in 0..text.width() as i32 {
      let mut outline_alpha = 0_u8;
      for offset_y in -radius..=radius {
        for offset_x in -radius..=radius {
          if offset_x * offset_x + offset_y * offset_y > radius_squared {
            continue;
          }
          let sample_x = x + offset_x;
          let sample_y = y + offset_y;
          if sample_x < 0
            || sample_y < 0
            || sample_x >= text.width() as i32
            || sample_y >= text.height() as i32
          {
            continue;
          }
          outline_alpha = outline_alpha.max(
              text.get_pixel(sample_x as u32, sample_y as u32)[3],
          );
        }
      }
      let alpha = ( u16::from(outline_alpha) * u16::from(outline_color[3]) / 255 ) as u8;
      outlined.put_pixel(
        x as u32,
        y as u32,
        Rgba([
          outline_color[0],
          outline_color[1],
          outline_color[2],
          alpha,
        ]),
      );
    }
  }
  // Draw the original antialiased text over the outline.
  for (x, y, source) in text.enumerate_pixels() {
    alpha_over(outlined.get_pixel_mut(x, y), *source);
  }
  outlined
}

fn alpha_over(destination: &mut Rgba<u8>, source: Rgba<u8>) {
  let source_alpha = source[3] as f32 / 255.0;
  let destination_alpha = destination[3] as f32 / 255.0;
  let output_alpha = source_alpha + destination_alpha * (1.0 - source_alpha);
  if output_alpha <= 0.0 {
    *destination = Rgba([0, 0, 0, 0]);
    return;
  }
  for channel in 0..3 {
    let source_color = source[channel] as f32 / 255.0;
    let destination_color = destination[channel] as f32 / 255.0;
    let output_color = (
      source_color * source_alpha
      + destination_color
      * destination_alpha
      * (1.0 - source_alpha)
    ) / output_alpha;
    destination[channel] = (output_color * 255.0).round() as u8;
  }
  destination[3] = (output_alpha * 255.0).round() as u8;
}

const EMOTE_PLACEHOLDER: &str = "\u{2003}";
const MSG_PTR: &str = "~> ";
const MIN_WIDTH: u32 = 40;
pub const LOREM_IPSUM: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, \
sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim \
veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. \
Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat \
nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia \
deserunt mollit anim id est laborum.";

