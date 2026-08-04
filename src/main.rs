// This file is just here for testing without running the plugin.
use {
  std::{
    io::{BufRead, BufReader, Read, Write, },
    net::{TcpListener, TcpStream, },
  },
};

const TWITCH_AUTH_URL: &str = "https://id.twitch.tv/oauth2/authorize?response_type=token&client_id=m0kk7y5gjs9qjfio2pw7hkw8iwaeft&redirect_uri=http://localhost:3000&scope=chat%3Aedit%20chat%3Aread";

const CALLBACK_PAGE: &str = r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <title>emojikanban Twitch authentication</title>
  </head>
  <body>
    <p id="status">Finishing Twitch authentication…</p>
    <script>
      const status = document.getElementById("status");
      const fragment = new URLSearchParams(window.location.hash.slice(1));
      const token = fragment.get("access_token");

      if (token === null) {
        status.textContent = "Twitch did not return an access token. You may close this window.";
      } else {
        fetch("/token", {
          method: "POST",
          headers: { "Content-Type": "text/plain" },
          body: token,
        }).then(response => {
          if (!response.ok) throw new Error(`HTTP ${response.status}`);
          status.textContent = "Authentication received. You may close this window.";
          history.replaceState(null, "", "/");
        }).catch(error => {
          status.textContent = `Failed to send authentication to emojikanban: ${error}`;
        });
      }
    </script>
  </body>
</html>
"#;

fn main() -> Result<(), anyhow::Error> {
  env_logger::Builder::from_default_env()
    .filter(None, log::LevelFilter::Info)
    .init();

  print!("Update Twitch authentication? (y/N) ");
  std::io::stdout().flush()?;
  
  let mut answer = String::new();
  std::io::stdin().read_line(&mut answer)?;
  let oauth: Option<String> = if answer.trim().eq_ignore_ascii_case("y") {
    let access_token = receive_twitch_access_token()?;
    println!("Twitch access token: {access_token}");
    Some(access_token)
  } else {
    None
  };
  
  let runtime = tokio::runtime::Runtime::new().unwrap();
  let (ekb_config_dirs, conf) = runtime.block_on(async {
    match emojikanban::get_or_create_config_emojikanban(oauth).await {
      Err(e)  => { Err(anyhow::format_err!("{}", e)) }
      Ok(res) => { Ok(res) }
    }
  })?;
  
  let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<emojikanban::plugin::EmoteData>();
  runtime.spawn(async move {
    if let Err(e) = emojikanban::start_twitch_monitor(ekb_config_dirs, conf, tx).await {
      log::error!("Twitch monitor died: {}", e);
    };
  });
  while let Some(emote_data) = rx.blocking_recv() {
    println!("Emote :{}: used.", emote_data.name);
  }
  
  Ok(())
}

fn receive_twitch_access_token() -> Result<String, anyhow::Error> {
  let listener = TcpListener::bind("127.0.0.1:3000")?;
  println!("Open this URL in your browser:\n{TWITCH_AUTH_URL}");

  for stream in listener.incoming() {
    let mut stream = stream?;
    let request = read_request(&mut stream)?;

    match (request.method.as_str(), request.path.as_str()) {
      ("GET", "/") => write_response(&mut stream, "200 OK", "text/html; charset=utf-8", CALLBACK_PAGE)?,
      ("POST", "/token") => {
        let token = String::from_utf8(request.body)?;
        let token = token.trim().to_owned();
        if token.is_empty() {
          write_response(&mut stream, "400 Bad Request", "text/plain; charset=utf-8", "Missing access token")?;
        } else {
          write_response(&mut stream, "200 OK", "text/plain; charset=utf-8", "Authentication received")?;
          return Ok(token);
        }
      }
      _ => write_response(&mut stream, "404 Not Found", "text/plain; charset=utf-8", "Not found")?,
    }
  }

  Err(anyhow::anyhow!("Twitch authentication server stopped before receiving a token"))
}

struct HttpRequest {
  method: String,
  path: String,
  body: Vec<u8>,
}

fn read_request(stream: &mut TcpStream) -> Result<HttpRequest, anyhow::Error> {
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

fn write_response(
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
