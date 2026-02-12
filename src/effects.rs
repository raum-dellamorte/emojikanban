use {
  enterpolation::{
    easing::{
      smootherstep,
    },
  },
  glam::{
    f32::{
      Vec2,
      // vec2,
    },
  },
  obs_wrapper::graphics::*,
  rand::prelude::*, 
};

pub trait EmoteEffect {
  fn update_dimensions(&mut self, w: f32, h: f32);
  fn update(&mut self, seconds: f32);
  fn draw(&self, tex: &GraphicsTexture);
  fn is_alive(&self) -> bool;
  // fn life_total(&self) -> f32;
  // fn life_lived(&self) -> f32;
  // fn pos(&self) -> &Vec2;
  // fn pos_mut(&mut self) -> &mut Vec2;
  // fn vel(&self) -> &Vec2;
  // fn vel_mut(&mut self) -> &mut Vec2;
  // fn scl(&self) -> &Vec2;
  // fn scl_mut(&mut self) -> &mut Vec2;
}

pub struct GravityEffect {
  screen_w: f32,
  screen_h: f32,
  emote_w: f32,
  emote_h: f32,
  life_total: f32,
  life_lived: f32,
  g: f32,
  bounce: f32,
  pos: Vec2,
  vel: Vec2,
  scl: Vec2,
}

impl GravityEffect {
  pub fn init(screen_w: f32, screen_h: f32, emote_w: f32, emote_h: f32, gravity: f32, bounce: f32, rng: &mut ThreadRng) -> Box<dyn EmoteEffect + 'static> {
    let mut pos = Vec2::ZERO;
    let mut vel = Vec2::ZERO;
    let scl = Vec2::ONE;
    pos.x = rng.random_range(0.1..0.9) as f32 * screen_w;
    vel.x = rng.random_range(-0.15..0.15) * screen_w;
    let life_total = rng.random_range(2.0..5.0);
    Box::new(Self {
      screen_w,
      screen_h,
      emote_w,
      emote_h,
      life_total,
      life_lived: 0.,
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
    self.life_lived += seconds;
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
  fn draw(&self, tex: &GraphicsTexture) {
    tex.draw(self.pos.x as i32, self.pos.y as i32, 0, 0, false);
  }
  fn is_alive(&self) -> bool { self.life_lived < self.life_total }
  // fn life_total(&self) -> f32 {
  //   self.life_total
  // }
  // fn life_lived(&self) -> f32 {
  //   self.life_lived
  // }
  // fn pos(&self) -> &Vec2 {
  //   &self.pos
  // }
  // fn pos_mut(&mut self) -> &mut Vec2 {
  //   &mut self.pos
  // }
  // fn vel(&self) -> &Vec2 {
  //   &self.vel
  // }
  // fn vel_mut(&mut self) -> &mut Vec2 {
  //   &mut self.vel
  // }
  // fn scl(&self) -> &Vec2 {
  //   &self.scl
  // }
  // fn scl_mut(&mut self) -> &mut Vec2 {
  //   &mut self.scl
  // }
}

pub struct SlideUpEffect {
  screen_w: f32,
  screen_h: f32,
  emote_w: f32,
  emote_h: f32,
  is_alive: bool,
  up_pause_down: [f32; 3],
  frame_time: f32,
  frame: usize,
  pos: Vec2,
  scl: Vec2,
}

impl SlideUpEffect {
  pub fn init(screen_w: f32, screen_h: f32, emote_w: f32, emote_h: f32, rng: &mut ThreadRng) -> Box<dyn EmoteEffect + 'static> {
    let mut pos = Vec2::ZERO;
    let scl = Vec2::from_array([512. / emote_w, 512. / emote_w]);
    pos.x = rng.random_range(0.15..=0.75) as f32 * screen_w;
    Box::new(Self {
      screen_w,
      screen_h,
      emote_w,
      emote_h,
      is_alive: true,
      up_pause_down: [3.,2.,3.],
      frame_time: 0.,
      frame: 0,
      pos, scl,
    })
  }
}

impl EmoteEffect for SlideUpEffect {
  fn update_dimensions(&mut self, w: f32, h: f32) {
    (self.screen_w, self.screen_h) = (w, h);
  }
  fn update(&mut self, seconds: f32) {
    self.frame_time += seconds;
    if self.frame_time > self.up_pause_down[self.frame] {
      self.frame += 1;
      self.frame_time = 0.;
    }
    if self.frame > 2 {
      self.is_alive = false;
      return;
    }
    let pct = self.frame_time / self.up_pause_down[self.frame];
    let step = smootherstep(pct);
    match self.frame {
      0 => {
        self.pos.y = self.screen_h - (self.emote_h * self.scl.y * step);
      }
      1 => {
        self.pos.y = self.screen_h - (self.emote_h * self.scl.y);
      }
      2 => {
        self.pos.y = self.screen_h - (self.emote_h * self.scl.y * (1. - step));
      }
      _ => { unreachable!() }
    }
  }
  fn draw(&self, tex: &GraphicsTexture) {
    tex.draw(self.pos.x as i32, self.pos.y as i32, (self.emote_w * self.scl.x) as u32, (self.emote_h * self.scl.y) as u32, false);
  }
  fn is_alive(&self) -> bool {
    self.is_alive
  }
  // fn life_total(&self) -> f32 {
  //   self.life_total
  // }
  // fn life_lived(&self) -> f32 {
  //   self.life_lived
  // }
  // fn pos(&self) -> &Vec2 {
  //   &self.pos
  // }
  // fn pos_mut(&mut self) -> &mut Vec2 {
  //   &mut self.pos
  // }
  // fn vel(&self) -> &Vec2 {
  //   &self.vel
  // }
  // fn vel_mut(&mut self) -> &mut Vec2 {
  //   &mut self.vel
  // }
  // fn scl(&self) -> &Vec2 {
  //   &self.scl
  // }
  // fn scl_mut(&mut self) -> &mut Vec2 {
  //   &mut self.scl
  // }
}
