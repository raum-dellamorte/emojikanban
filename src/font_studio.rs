// #![allow(dead_code,unused)]
use {
  crate::{
    ChatData,
    plugin::EmoteOBS,
  },
  cosmic_text::{
    Attrs, AttrsOwned, Buffer, Color, FeatureTag, FontFeatures, FontSystem, Metrics, Shaping, SwashCache, Weight,
  },
  image::{
    // AnimationDecoder, DynamicImage, ImageFormat,
    Rgba, RgbaImage,
    // imageops::{
    //   FilterType,
    //   resize, overlay,
    // }
    // codecs::gif::GifDecoder,
  },
  obs_wrapper::graphics::*,
  std::{
    cell::RefCell,
    collections::VecDeque,
    rc::Rc,
    sync::Arc,
  },
};

pub struct FontStudio {
  font_system: FontSystem,
  swash_cache: SwashCache,
  buffer: Buffer,
  attrs: AttrsOwned,
  pub text_blocks: VecDeque<TextBlock>,
  pub chat_blocks: VecDeque<ChatDataWithBlock>,
  user_metrics: (f32,f32),
  bg_color: Color,
  bg_rounding: f32,
  redraw_bg_timer: Option<f32>,
  redraw_chat_timer: Option<f32>,
  bg_tex: Option<GraphicsTexture>,
  text_color: Color,
  outline_color: Color,
  msg_ptr_color: Color,
  chat_life: f32,
  chat_w: u32,
  chat_h: u32,
  chat_offset: (i32,i32),
  chat_metrics: (f32,f32),
  chat_margin: i32,
  pub always_draw_bg: bool,
}

impl FontStudio {
  pub fn new(chat_w: u32, chat_h: u32) -> Self {
    let mut font_system = FontSystem::new();
    let swash_cache = SwashCache::new();
    let buffer = Buffer::new(&mut font_system, Metrics::new(20.0, 24.0)); // The metrics here don't matter, we reset it when text is added
    let attrs = Attrs::new();
    let attrs = AttrsOwned::new(&attrs);
    let bg_color = Color::rgba(0x0A, 0x32, 0x0A, 0xB3);
    let text_color = Color::rgba(0xC8, 0xC8, 0xC8, 0xFF);
    let outline_color = Color::rgba(0xAA, 0x00, 0x00, 0xFF);
    let msg_ptr_color = Color::rgba(0x36, 0x87, 0x77, 0xFF);
    let cbg_img = create_chat_bg(chat_w,chat_h,20.0,bg_color.as_rgba());
    let bg_tex = Some(gen_rgba_tex(cbg_img));
    Self {
      font_system,
      swash_cache,
      buffer,
      attrs,
      text_blocks: VecDeque::new(),
      chat_blocks: VecDeque::new(),
      user_metrics: (26.0, 30.0),
      bg_color,
      bg_rounding: 20.0,
      redraw_bg_timer: None,
      redraw_chat_timer: None,
      bg_tex,
      text_color,
      outline_color,
      msg_ptr_color,
      chat_life: 120.0,
      chat_w,
      chat_h,
      chat_offset: (0,0),
      chat_metrics: (30.0,34.0),
      chat_margin: 10,
      always_draw_bg: false,
    }
  }
  pub fn update_dimensions(&mut self, w: u32, h: u32) {
    if self.chat_w == w && self.chat_h == h { return; }
    (self.chat_w, self.chat_h) = (w, h);
    self.redraw_bg_timer = Some(0.5);
    self.redraw_chat_timer = Some(0.5);
  }
  pub fn update_bg_color(&mut self, color: [u8;4]) {
    if self.bg_color.as_rgba() == color { return; }
    self.bg_color = Color::rgba(color[0], color[1], color[2], color[3]);
    self.redraw_bg_timer = Some(0.5);
  }
  pub fn update_bg_color_u32(&mut self, color: u32) {
    self.update_bg_color(rgba_array_from_u32(color));
  }
  pub fn update_text_color(&mut self, color: [u8;4]) {
    if self.text_color.as_rgba() == color { return; }
    self.text_color = Color::rgba(color[0], color[1], color[2], color[3]);
    log::info!("New text color {:?}", self.text_color.as_rgba());
    self.redraw_chat_timer = Some(0.5);
  }
  pub fn update_text_color_u32(&mut self, color: u32) {
    self.update_text_color(rgba_array_from_u32(color));
  }
  pub fn update_outline_color(&mut self, color: [u8;4]) {
    if self.outline_color.as_rgba() == color { return; }
    self.outline_color = Color::rgba(color[0], color[1], color[2], color[3]);
    log::info!("New outline color {:?}", self.outline_color.as_rgba());
    self.redraw_chat_timer = Some(0.5);
  }
  pub fn update_outline_color_u32(&mut self, color: u32) {
    self.update_outline_color(rgba_array_from_u32(color));
  }
  pub fn update_msg_ptr_color(&mut self, color: [u8;4]) {
    if self.msg_ptr_color.as_rgba() == color { return; }
    self.msg_ptr_color = Color::rgba(color[0], color[1], color[2], color[3]);
    log::info!("New message pointer color {:?}", self.msg_ptr_color.as_rgba());
    self.redraw_chat_timer = Some(0.5);
  }
  pub fn update_msg_ptr_color_u32(&mut self, color: u32) {
    self.update_msg_ptr_color(rgba_array_from_u32(color));
  }
  pub fn redraw_bg_texture(&mut self) {
    if self.redraw_bg_timer.is_none() || self.redraw_bg_timer.unwrap() > 0.0 { return; }
    self.redraw_bg_timer = None;
    let cbg_img = create_chat_bg(self.chat_w,self.chat_h,self.bg_rounding,self.bg_color.as_rgba());
    self.bg_tex = Some(gen_rgba_tex(cbg_img));
  }
  pub fn rebuild_chats(&mut self) {
    if self.redraw_chat_timer.is_none() || self.redraw_chat_timer.unwrap() > 0.0 { return; }
    self.redraw_chat_timer = None;
    let mut chats = std::mem::take(&mut self.chat_blocks);
    for msg in &mut chats {
      msg.block = None;
      self.build_chat_msg(msg);
    }
    self.chat_blocks = chats;
  }
  pub fn update(&mut self, seconds: f32) {
    self.text_blocks.retain(|tblk| tblk.is_alive() );
    self.chat_blocks.retain(|cblk| cblk.is_alive() );
    if let Some(timer) = self.redraw_bg_timer.as_mut() {
      *timer -= seconds;
    }
    if let Some(timer) = self.redraw_chat_timer.as_mut() {
      *timer -= seconds;
    }
    for tblk in self.text_blocks.iter_mut() {
      tblk.update(seconds);
    }
    let mut current_y = self.chat_offset.1 + self.chat_margin;
    for cdata in self.chat_blocks.iter_mut() {
      if cdata.block.is_none() { continue; }
      cdata.set_y(current_y);
      cdata.update(seconds);
      current_y += cdata.height() as i32;
    }
    self.redraw_bg_texture();
    self.rebuild_chats();
  }
  pub fn add_text_block(&mut self, image_width: u32, offset: (i32,i32), metrics: (f32,f32), life: Option<f32>, txt: &str) {
    let mut buffer = self.buffer.borrow_with(&mut self.font_system);
    let (x_offset, y_offset) = offset;
    let img_w = image_width.max(MIN_WIDTH);
    let inner_w = img_w - (2 * self.chat_margin as u32);
    buffer.set_size(Some(inner_w as f32), None);
    buffer.set_metrics(Metrics::new(metrics.0, metrics.1));
    buffer.set_text(txt, &self.attrs.as_attrs(), Shaping::Advanced, None);
    buffer.shape_until_scroll(false);
    let img_h = buffer.layout_runs().map(|run| run.line_top + run.line_height)
      .fold(0.0f32, f32::max).ceil() as u32 + (2 * self.chat_margin as u32);
    let mut img = RgbaImage::from_pixel(img_w, img_h, Rgba([0,0,0,0]));
    buffer.draw(&mut self.swash_cache, self.text_color, draw_buffer(&mut img, self.chat_margin));
    let tex = gen_rgba_tex(img);
    let tblk = TextBlock { tex, life, x_offset, y_offset, };
    self.text_blocks.push_back(tblk);
  }
  pub fn add_chat_msg(&mut self, msg: Arc<ChatData>) {
    let mut msg = ChatDataWithBlock { data: msg, block: None, life: Some(self.chat_life) };
    self.build_chat_msg(&mut msg);
    self.chat_blocks.push_back(msg);
  }
  fn build_chat_msg(&mut self, chat_data: &mut ChatDataWithBlock) {
    let mut buffer = self.buffer.borrow_with(&mut self.font_system);
    let (x_offset, y_offset) = self.chat_offset;
    let (font_size, line_height) = self.chat_metrics;
    let img_w = self.chat_w.max(MIN_WIDTH);
    let inner_w = img_w - (2 * self.chat_margin) as u32;
    let msg = chat_data.data.clone();
    let user: &str = &msg.user;
    let user_attrs = Attrs::new()
      .color(msg.uname_color.unwrap_or(Color::rgb(0xC8, 0x64, 0xC8))) // Default username color u32 0xFFC864C8
      .font_features(FontFeatures::new().enable(FeatureTag::SMALL_CAPS).to_owned())
      .metrics(Metrics::new(self.user_metrics.0, self.user_metrics.1))
      .weight(Weight::BOLD);
    let msg_ptr_attrs = Attrs::new()
      .color(self.msg_ptr_color)
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
    buffer.draw(&mut self.swash_cache, self.text_color, draw_buffer(&mut usr_img, self.chat_margin));
    let usr_tex = gen_rgba_tex(usr_img);
    // 
    // Generate chat message image
    // 
    let msg_string: String = if msg.emotes.len() == 0 { msg.msg.to_owned() } else {
      let mut filtered = String::with_capacity(msg.msg.len());
      let mut i = 0;
      for emote in msg.emotes.iter() {
        if emote.loc.0 >= i {
          filtered.push_str( &msg.msg[i..emote.loc.0] );
        }
        filtered.push_str( EMOTE_MARKER );
        if emote.loc.1 > i { i = emote.loc.1; }
      }
      if i < msg.msg.len() { filtered.push_str( &msg.msg[i..] ); }
      filtered
    };
    let msg_txt: &str = &msg_string;
    let msg_attrs = Attrs::new().color(self.text_color);
    buffer.set_size(Some((inner_w as i32 - msg_indent) as f32), None);
    let emote_size = self.chat_metrics.1.ceil() as u32;
    let emote_size = emote_size.saturating_sub(1);
    let emote_attrs = Attrs::new().color(
      Color::rgba(0, 0, 0, 0)
    ).metrics(
      Metrics::new(emote_size as f32, emote_size as f32)
    );
    let mut emote_pos = vec![None;msg.emotes.len()];
    let img_h = if msg.emotes.len() == 0 {
      buffer.set_text(msg_txt, &msg_attrs, Shaping::Advanced, None);
      buffer.shape_until_scroll(false);
      buffer.layout_runs().map(|run| run.line_top + run.line_height)
        .fold(0.0f32, f32::max).ceil() as u32 + (2 * self.chat_margin as u32)
    } else {
      let mut parts = msg_string.split(EMOTE_MARKER).peekable();
      let mut spans = Vec::new();
      let mut emote_index = 0;
      while let Some(text) = parts.next() {
        if !text.is_empty() {
          spans.push((text, msg_attrs.clone()));
        }
        if parts.peek().is_some() {
          spans.push((EMOTE_BREAK, msg_attrs.clone()));
          spans.push((EMOTE_GLYPH, emote_attrs.clone().metadata(emote_index + 1)));
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
      let mut out = 0.0_f32;
      for run in buffer.layout_runs() {
        out = out.max(run.line_top + run.line_height);
        for glyph in run.glyphs {
          let Some(emote_index) = glyph.metadata.checked_sub(1) else { continue; };
          let Some(pos) = emote_pos.get_mut(emote_index) else {
            log::error!("Emote metadata {} has no matching chat emote", glyph.metadata);
            continue;
          };
          let x = self.chat_margin + glyph.x.round() as i32;
          let y = self.chat_margin + (
            run.line_top + (run.line_height - emote_size as f32) / 2.0
          ).round() as i32;
          *pos = Some((x,y));
        }
      }
      out.ceil() as u32 + (2 * self.chat_margin as u32)
    };
    let inline_emotes = msg.emotes.iter().cloned().zip(emote_pos.clone()).filter_map(|(emote_data, pos)| {
      let local = pos?;
      let emote: EmoteOBS = emote_data.into();
      if !emote.has_frames() {
        log::error!(
          "No decoded frames for inline emote {}",
          emote.name,
        );
        return None;
      }
      Some(ChatInlineEmote {emote, local})
    }).collect();
    let mut msg_img = RgbaImage::from_pixel(inner_w, img_h, Rgba([0,0,0,0]));
    // let text_color = Color::rgb(0xFF, 0xFF, 0xFF);
    buffer.draw(&mut self.swash_cache, self.text_color, draw_buffer(&mut msg_img, self.chat_margin));
    let msg_img = add_text_outline(&msg_img, 2, Rgba(self.outline_color.as_rgba()));
    let msg_tex = gen_rgba_tex(msg_img);
    let cblk = ChatMsgBlock {
      usr_tex, msg_tex, inline_emotes, emote_size,
      x_offset, y_offset,
      msg_y_offset: self.user_metrics.1 as i32,
      msg_indent, 
    };
    chat_data.block = Some(Rc::new(RefCell::new(cblk)));
  }
  pub fn draw(&self) {
    if (self.always_draw_bg || !self.chat_blocks.is_empty()) && let Some(bg) = self.bg_tex.as_ref() {
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

#[derive(Clone)]
pub struct ChatDataWithBlock {
  data: Arc<ChatData>,
  block: Option<Rc<RefCell<ChatMsgBlock>>>,
  life: Option<f32>,
}

pub struct ChatMsgBlock {
  usr_tex: GraphicsTexture,
  msg_tex: GraphicsTexture,
  inline_emotes: Vec<ChatInlineEmote>,
  emote_size: u32,
  pub x_offset: i32,
  pub y_offset: i32,
  msg_y_offset: i32,
  msg_indent: i32,
}

impl FontStudioTextBlock for ChatDataWithBlock {
  fn draw(&self) {
    let Some(block) = self.block.as_ref() else { return; };
    let cblk = block.borrow();
    cblk.usr_tex.draw(cblk.x_offset, cblk.y_offset, 0, 0, false);
    let msg_global_x = cblk.x_offset + cblk.msg_indent;
    let msg_global_y = cblk.y_offset + cblk.msg_y_offset;
    cblk.msg_tex.draw(msg_global_x, msg_global_y, 0, 0, false);
    for emote in cblk.inline_emotes.iter() {
      let (x,y) = emote.local;
      emote.emote.current_frame().draw(
        msg_global_x + x, msg_global_y + y, cblk.emote_size, cblk.emote_size, false);
    }
  }
  fn is_alive(&self) -> bool {
    self.life.is_none_or(|life| life > 0.0)
  }
  fn height(&self) -> u32 {
    let Some(block) = self.block.as_ref() else { return 0; };
    let cblk = block.borrow();
    cblk.msg_y_offset as u32 + cblk.msg_tex.height()
  }
  fn update(&mut self, seconds: f32) {
    if let Some(life) = self.life.as_mut() {
      *life -= seconds;
    }
    if let Some(block) = self.block.as_ref() {
      for emote in block.borrow_mut().inline_emotes.iter_mut() {
        emote.emote.update(seconds);
      }
    }
  }
  fn set_x(&mut self, val: i32) {
    if let Some(block) = self.block.as_ref() {
      block.borrow_mut().x_offset = val;
    }
  }
  fn set_y(&mut self, val: i32) {
    if let Some(block) = self.block.as_ref() {
      block.borrow_mut().y_offset = val;
    }
  }
}

pub struct ChatInlineEmote {
  emote: EmoteOBS,
  local: (i32, i32),
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

pub struct ColorConverter<T>(pub Option<T>);

impl From<Option<twitch_message::Color>> for ColorConverter<Color> {
  fn from(value: Option<twitch_message::Color>) -> Self {
    if let Some(c) = value {
      ColorConverter(Some(Color::rgb(c.0, c.1, c.2)))
    } else { ColorConverter(None) }
  }
}

/// Convert OBS u32 color to rgba u8 array
/// 
/// Note: OBS displays colors as #AARRGGBB but seems to format the u32 as AABBGGRR
pub fn rgba_array_from_u32(color: u32) -> [u8;4] {
  let r =  color        as u8;
  let g = (color >>  8) as u8;
  let b = (color >> 16) as u8;
  let a = (color >> 24) as u8;
  [r,g,b,a]
}

pub fn rgba_to_obs_u32(color: [u8;4]) -> u32 {
  color[0] as u32 | ((color[1] as u32) << 8) | ((color[2] as u32) << 16) | ((color[3] as u32) << 24)
}

// const EMOTE_PLACEHOLDER: &str = "\u{2003}";
const EMOTE_MARKER: &str = "\u{FFFC}";
const EMOTE_BREAK: &str = "\u{200B}";
const EMOTE_GLYPH: &str = "\u{25A1}";
const MSG_PTR: &str = "~> ";
const MIN_WIDTH: u32 = 40;
pub const LOREM_IPSUM: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, \
sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim \
veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. \
Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat \
nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia \
deserunt mollit anim id est laborum.";

