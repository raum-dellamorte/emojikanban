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
  rand::{
    // Rng,
    prelude::*,
  }, 
  rand_distr::{
    Distribution,
    UnitCircle,
  },
};

pub trait EmoteEffect {
  fn update_dimensions(&mut self, w: f32, h: f32);
  fn update(&mut self, seconds: f32);
  fn draw(&self, tex: &GraphicsTexture);
  fn is_alive(&self) -> bool;
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
      screen_w, screen_h,
      emote_w, emote_h,
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
      screen_w, screen_h,
      emote_w, emote_h,
      is_alive: true,
      up_pause_down: [3.,2.,3.],
      frame_time: 0.,
      frame: 0,
      pos, scl,
    })
  }
  fn x_scale(&self) -> u32 { (self.emote_w * self.scl.x) as u32 }
  fn y_scale(&self) -> u32 { (self.emote_h * self.scl.y) as u32 }
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
    tex.draw(self.pos.x as i32, self.pos.y as i32, self.x_scale(), self.y_scale(), false);
  }
  fn is_alive(&self) -> bool {
    self.is_alive
  }
}

pub struct InchWormEffect {
  screen_w: f32,
  screen_h: f32,
  emote_w: f32,
  emote_h: f32,
  is_alive: bool,
  segments: [Vec2; 9],
  target: Vec2,
  // direction: Vec2,
  step: Vec2,
  scl: Vec2,
  frame_time: f32,
  step_time: f32,
  life_counter: usize,
  move_head: bool,
}

impl InchWormEffect {
  pub fn init(screen_w: f32, screen_h: f32, emote_w: f32, emote_h: f32, rng: &mut ThreadRng) -> Box<dyn EmoteEffect + 'static> {
    let unit_dir: [f32; 2] = UnitCircle.sample(rng);
    let direction = Vec2::from_array(unit_dir);
    let step = direction * (56. * 9.);
    let target = step;
    let scl = Vec2::from_array([128. / emote_w, 128. / emote_w]);
    Box::new(Self {
      screen_w, screen_h,
      emote_w, emote_h,
      is_alive: true,
      segments: [Vec2::default(); 9],
      target,
      // direction,
      step,
      scl,
      frame_time: 0.,
      life_counter: 0,
      step_time: 1.0,
      move_head: true,
    })
  }
  fn x_scale(&self) -> u32 { (self.emote_w * self.scl.x) as u32 }
  fn y_scale(&self) -> u32 { (self.emote_h * self.scl.y) as u32 }
  fn seg_tail(&self) -> &Vec2 {
    &self.segments[0]
  }
  fn seg_head(&self) -> &Vec2 {
    &self.segments[self.segments.len() - 1]
  }
  fn seg_tail_mut(&mut self) -> &mut Vec2 {
    &mut self.segments[0]
  }
  fn seg_head_mut(&mut self) -> &mut Vec2 {
    &mut self.segments[self.segments.len() - 1]
  }
}

impl EmoteEffect for InchWormEffect {
  fn update_dimensions(&mut self, w: f32, h: f32) {
    (self.screen_w, self.screen_h) = (w, h);
  }
  fn update(&mut self, seconds: f32) {
    self.frame_time += seconds;
    let len = self.segments.len();
    let (step_pct, move_head) = if self.frame_time >= self.step_time {
      self.frame_time = 0.0;
      self.move_head = !self.move_head;
      if self.move_head {
        self.life_counter += 1;
        self.target += self.step;
        if self.life_counter > 3 { self.is_alive = false; }
      }
      (1.0, !self.move_head) 
    } else { (smootherstep(self.frame_time / self.step_time), self.move_head) };
    if move_head {
      let pos = self.seg_tail() + (self.step * step_pct);
      self.seg_head_mut().clone_from(&pos);
      for i in 1..(len - 1) {
        let pct = smootherstep((i as f32 / (len - 1) as f32) * step_pct);
        self.segments[i] = self.seg_tail() + (self.step * pct);
      }
    } else {
      let pos = self.seg_head() - (self.step * (1.0 - step_pct));
      self.seg_tail_mut().clone_from(&pos);
      for i in 1..(len - 1) {
        let pct = smootherstep((len - i) as f32 / (len - 1) as f32) * (1.0 - step_pct);
        self.segments[i] = self.seg_head() - (self.step * pct);
      }
    }
  }
  fn draw(&self, tex: &GraphicsTexture) {
    let hw = self.screen_w as i32 / 2;
    let hh = self.screen_h as i32 / 2;
    for seg in self.segments.iter() {
      tex.draw(seg.x as i32 + hw, seg.y as i32 + hh, self.x_scale(), self.y_scale(), false);
    }
  }
  fn is_alive(&self) -> bool {
    self.is_alive
  }
}

