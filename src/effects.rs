use {
  glam::{
    f32::{
      Vec2,
      // vec2,
    },
  },
  rand::prelude::*, 
};

pub trait EmoteEffect {
  fn update_dimensions(&mut self, w: f32, h: f32);
  fn update(&mut self, seconds: f32);
  fn pos(&self) -> &Vec2;
  fn pos_mut(&mut self) -> &mut Vec2;
  fn vel(&self) -> &Vec2;
  fn vel_mut(&mut self) -> &mut Vec2;
  fn scl(&self) -> &Vec2;
  fn scl_mut(&mut self) -> &mut Vec2;
}

pub struct GravityEffect {
  pub screen_w: f32,
  pub screen_h: f32,
  pub emote_w: f32,
  pub emote_h: f32,
  pub g: f32,
  pub bounce: f32,
  pub pos: Vec2,
  pub vel: Vec2,
  pub scl: Vec2,
}

impl GravityEffect {
  pub fn init(screen_w: f32, screen_h: f32, emote_w: f32, emote_h: f32, gravity: f32, bounce: f32, rng: &mut ThreadRng) -> Box<dyn EmoteEffect + 'static> {
    let mut pos = Vec2::ZERO;
    let mut vel = Vec2::ZERO;
    let scl = Vec2::ONE;
    pos.x = rng.random_range(0.1..0.9) as f32 * screen_w;
    vel.x = rng.random_range(-0.15..0.15) * screen_w;
    Box::new(Self {
      screen_w,
      screen_h,
      emote_w,
      emote_h,
      g: gravity,
      bounce,
      pos, vel, scl,
    })
  }
}

impl EmoteEffect for GravityEffect {
  fn update_dimensions(&mut self, w: f32, h: f32) {
    (self.screen_w, self.screen_h) = (w, h);
  }
  fn update(&mut self, seconds: f32) {
    let (w, h) = (self.screen_w, self.screen_h);
    let ew = self.emote_w * self.scl.x;
    let eh = self.emote_h * self.scl.y;
    let x = &mut self.pos.x;
    let y = &mut self.pos.y;
    let vx = &mut self.vel.x;
    let vy = &mut self.vel.y;
    // Update velocity
    *vy += self.g * seconds;
    // Apply velocity
    *x += *vx * seconds;
    *y += *vy * seconds;
    // Bounce
    let floor: f32 = h - eh;
    if *y > floor {
      *y = floor;
      *vy = -*vy * self.bounce;
    }
    if *x < 0. || *x >= w - ew {
      *vx = -*vx;
    }
  }
  fn pos(&self) -> &Vec2 {
    &self.pos
  }
  fn pos_mut(&mut self) -> &mut Vec2 {
    &mut self.pos
  }
  fn vel(&self) -> &Vec2 {
    &self.vel
  }
  fn vel_mut(&mut self) -> &mut Vec2 {
    &mut self.vel
  }
  fn scl(&self) -> &Vec2 {
    &self.scl
  }
  fn scl_mut(&mut self) -> &mut Vec2 {
    &mut self.scl
  }
}

