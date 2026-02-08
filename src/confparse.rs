use {
  nom::{
    AsChar, 
    IResult, 
    bytes::{
      complete::{
        tag,
        take_till,
      },
    }, 
    character::complete::{
      multispace0,
      space1,
    },
  },
};

#[derive(Debug)]
pub struct EkbConfig {
  bot_account: String,
  channel:     String,
  oauth:       String,
}
impl EkbConfig {
  pub fn bot_account(&self) -> String { self.bot_account.to_owned() }
  pub fn channel(&self) -> String { self.channel.to_owned() }
  pub fn oauth(&self) -> String { self.oauth.to_owned() }
}

pub fn parse_config(conf: &str) -> Result<EkbConfig, String> {
  match _parse_config(conf) {
    Ok((_, out)) => { Ok(out) }
    Err(e) => { Err(e.to_string()) }
  }
}

fn _parse_config(s: &str) -> IResult<&str, EkbConfig> {
  let (s, a) = _get_tagged_value("bot-account", s)?;
  let (s, b) = _get_tagged_value("channel", s)?;
  let (s, c) = _get_tagged_value("oauth", s)?;
  let bot_account = a.to_string();
  let channel = b.to_string();
  let oauth = c.to_string();
  Ok((s, EkbConfig {
    bot_account,
    channel,
    oauth,
  }))
}

fn _get_tagged_value<'a>(t: &'a str, s: &'a str) -> IResult<&'a str, &'a str> {
  let (s, _) = multispace0(s)?;
  let (s, _) = tag(t)(s)?;
  let (s, _) = space1(s)?;
  let (s, o) = take_till(AsChar::is_space)(s)?;
  let (s, _) = take_till(AsChar::is_newline)(s)?;
  Ok((s, o))
}
