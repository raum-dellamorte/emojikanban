use {
  kdl::{
    KdlDocument,
    // KdlEntry,
    KdlValue,
  },
  std::{
    path::PathBuf,
    io::{BufRead, BufReader, Read, Write, },
    net::{TcpListener, TcpStream, },
  },
};

#[derive(Debug, Clone)]
pub struct EkbConfigDirs {
  pub config: PathBuf,
  pub data:   PathBuf,
}

#[derive(Debug, Clone)]
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

pub const TWITCH_AUTH_URL: &str = "https://id.twitch.tv/oauth2/authorize?response_type=token&client_id=m0kk7y5gjs9qjfio2pw7hkw8iwaeft&redirect_uri=http://localhost:3000&scope=chat%3Aedit%20chat%3Aread";

pub const CALLBACK_PAGE: &str = r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <title>emojikanban Twitch authentication</title>
  </head>
  <body>
    <p id="status_1" style="text-align: center; font-size: clamp(2rem, 5vw, 3rem);">
      Use this
      <a href="https://id.twitch.tv/oauth2/authorize?response_type=token&client_id=m0kk7y5gjs9qjfio2pw7hkw8iwaeft&redirect_uri=http://localhost:3000&scope=chat%3Aedit%20chat%3Aread">link</a>
      to authorize EmojiKanBan with Twitch.
    </p>
    <p id="status_2" style="text-align: center; font-size: clamp(1rem, 2vw, 2rem);">Awaiting Twitch authentication…</p>
    <script>
      const status1 = document.getElementById("status_1");
      const status2 = document.getElementById("status_2");
      const fragment = new URLSearchParams(window.location.hash.slice(1));
      const token = fragment.get("access_token");
      if (token === null) {
        status2.textContent = "Awaiting Twitch authentication…";
      } else {
        fetch("/token", {
          method: "POST",
          headers: { "Content-Type": "text/plain" },
          body: token,
        }).then(response => {
          if (!response.ok) throw new Error(`HTTP ${response.status}`);
          status1.textContent = "Authentication received.";
          status2.textContent = "You may close this window.";
          history.replaceState(null, "", "/");
        }).catch(error => {
          status2.textContent = `Failed to send authentication to emojikanban: ${error}`;
        });
      }
    </script>
  </body>
</html>
"#;

pub fn serve_oauth_receiver() -> Result<String, anyhow::Error> {
  let listener = TcpListener::bind("127.0.0.1:3000")?;
  for stream in listener.incoming() {
    let mut stream = stream?;
    let request = oauth_rcvr_read_request(&mut stream)?;
    match (request.method.as_str(), request.path.as_str()) {
      ("GET", "/") => oauth_rcvr_write_response(&mut stream, "200 OK", "text/html; charset=utf-8", CALLBACK_PAGE)?,
      ("POST", "/token") => {
        let token = String::from_utf8(request.body)?;
        let token = token.trim().to_owned();
        if token.is_empty() {
          oauth_rcvr_write_response(&mut stream, "400 Bad Request", "text/plain; charset=utf-8", "Missing access token")?;
        } else {
          oauth_rcvr_write_response(&mut stream, "200 OK", "text/plain; charset=utf-8", "Authentication received")?;
          return Ok(token);
        }
      }
      _ => oauth_rcvr_write_response(&mut stream, "404 Not Found", "text/plain; charset=utf-8", "Not found")?,
    }
  }
  Err(anyhow::anyhow!("Twitch authentication server stopped before receiving a token"))
}

struct HttpRequest {
  method: String,
  path: String,
  body: Vec<u8>,
}

fn oauth_rcvr_read_request(stream: &mut TcpStream) -> Result<HttpRequest, anyhow::Error> {
  let mut reader = BufReader::new(stream);
  let mut request_line = String::new();
  reader.read_line(&mut request_line)?;
  let mut request_parts = request_line.split_whitespace();
  let method = request_parts.next().ok_or_else(|| anyhow::anyhow!("HTTP request has no method"))?.to_owned();
  let path = request_parts.next().ok_or_else(|| anyhow::anyhow!("HTTP request has no path"))?.to_owned();
  let mut content_length = 0;
  loop {
    let mut header = String::new();
    reader.read_line(&mut header)?;
    if header == "\r\n" || header == "\n" {
      break;
    }
    if let Some((name, value)) = header.split_once(':')
      && name.eq_ignore_ascii_case("content-length")
    {
      content_length = value.trim().parse()?;
    }
  }
  let mut body = vec![0; content_length];
  reader.read_exact(&mut body)?;
  Ok(HttpRequest { method, path, body })
}

fn oauth_rcvr_write_response(
  stream: &mut TcpStream,
  status: &str,
  content_type: &str,
  body: &str,
) -> std::io::Result<()> {
  write!(
    stream,
    "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
    body.len(),
  )?;
  stream.flush()
}

