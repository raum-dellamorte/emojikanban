use {
  kdl::{
    KdlDocument,
    // KdlEntry,
    KdlValue,
  },
  std::path::PathBuf,
};

#[derive(Debug)]
pub struct EkbConfigDirs {
  pub config: PathBuf,
  pub data:   PathBuf,
}

#[derive(Debug)]
pub struct EkbTwitchConfig {
  bot_account: String,
  channel:     String,
  oauth:       String,
}
impl EkbTwitchConfig {
  pub fn bot_account(&self) -> String { self.bot_account.to_owned() }
  pub fn channel(&self) -> String { self.channel.to_owned() }
  pub fn oauth(&self) -> String { self.oauth.to_owned() }
}

impl TryFrom<KdlDocument> for EkbTwitchConfig {
  type Error = String;
  fn try_from(conf: KdlDocument) -> Result<Self, Self::Error> {
    let oauth = match conf.oauth() {
      Err(e) => { return Err(e); }
      Ok(val) => { val }
    };
    let bot_account: String = match conf.bot_account() {
      Err(e) => { return Err(e); }
      Ok(val) => { val }
    };
    let channel = conf.channel();
    let channel = channel.unwrap_or(bot_account.clone());
    Ok(Self { bot_account, channel, oauth })
  }
}

pub trait EkbTwitchValues {
  fn bot_account(&self) -> Result<String, String>;
  fn channel(&self) -> Result<String, String>;
  fn oauth(&self) -> Result<String, String>;
  fn oauth_update(&mut self, new_oauth: &str) -> Result<(),String>;
}

#[allow(clippy::needless_return)]
impl EkbTwitchValues for KdlDocument {
  fn bot_account(&self) -> Result<String, String> {
    if let Some(node) = self.get("bot-account") {
      if let Some(entry) = node.entry(0) {
        match entry.value() {
          KdlValue::String(oauth) => {
            if oauth.len() >= 6 && &oauth[..6] == "oauth:" {
              Ok(oauth[6..].to_owned())
            } else {
              Ok(oauth.to_owned())
            }
          }
          e => { return Err(format!("bot_account node first entry should be the username of the bot account as a String. Found {:?}", e)); }
        }
      } else { return Err("bot_account node has no fields".to_owned()); }
    } else { return Err("bot_account node not present".to_owned()); }
  }
  fn channel(&self) -> Result<String, String> {
    if let Some(node) = self.get("channel") {
      if let Some(entry) = node.entry(0) {
        match entry.value() {
          KdlValue::String(val) => { Ok(val.to_owned()) }
          e => { return Err(format!("channel node first entry should be the username of the channel you want to connect to as a string. Found {:?}", e)); }
        }
      } else { return Err("channel node has no fields".to_owned()); }
    } else { return Err("channel node not present".to_owned()); }
  }
  fn oauth(&self) -> Result<String, String> {
    if let Some(node) = self.get("oauth") {
      if let Some(entry) = node.entry(0) {
        match entry.value() {
          KdlValue::String(val) => { Ok(val.to_owned()) }
          e => { return Err(format!("oauth node first entry should be the oauth access token as a string. Found {:?}", e)); }
        }
      } else { return Err("oauth node has no fields".to_owned()); }
    } else { return Err("oauth node not present".to_owned()); }
  }
  fn oauth_update(&mut self, new_oauth: &str) -> Result<(),String> {
    if let Some(node) = self.get_mut("oauth") {
      if let Some(entry) = node.entry_mut(0) {
        entry.set_value(new_oauth);
        let value_repr = entry.value().to_string();
        if let Some(format) = entry.format_mut() {
          format.value_repr = value_repr;
          Ok(())
        } else {
          return Err("internal entry updated but the string format representation was not. entry.format_mut() did not return Some(format)".to_owned())
        }
      } else { return Err("oauth node has no fields".to_owned()); }
    } else { return Err("oauth node not present".to_owned()); }
  }
}

