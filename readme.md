 エモジ看板 emojiKanban
========================

OBS plugin Emote Wall 
---------------------

A local emote wall written in Rust as an OBS Plugin. No HTML, No JavaScript, No Meta Cookies. The only external reliance is on Twitch.tv (not that I've read the code of any of the libraries propping this creation up). A step towards self hosting whatever possible on open source software.

Plugin gives new Source `emojikanban` which connects to Twitch via IRC and monitors chat for emotes to be drawn with some effect to the screen. There is currently only one effect. Any emote used in chat (unless it's a single emote by itself for some reason) spawns at the top of the screen, falls, and bounces. They live for between 2 and 5 seconds. There's currently near nothing to protect you from being overloaded with emotes. Safety concerns are planned to be addressed.

On first use, generates `~/.config/emojikanban/config.kdl` with dummy data to be replaced with `oauth` credentials, and while I believe it's valid KDL, it isn't parsed as KDL, so don't change the order or remove a key or value. There may be room for improvement, but the plugin really just needs three strings from the user. The `bot-account` used to generate the `oauth` token, the `channel` to connect to the chat of via IRC (generally your streamer account), and the `oauth` token, all as key value pairs in the config file parse with `nom` ~~for fun and profit~~. It may be parsed as actual KDL in the future as it may end up including other settings.

Emotes are stored in a local sqlite database stored in `~/.config/emojikanban/emotes.db3` (untested on Windows) so that they are only downloaded once.

Use at your own risk :)

```bash
cargo build -r
sudo ln -s $(pwd)/target/release/libemojikanban.so /usr/lib/obs-plugins/
```

`~/.config/emojikanban/config.kdl`:
```kdl
bot-account bot-name                       // <- Replace 'bot-name' with the name of the account used to monitor chat
channel     streamer-name                  // <- and 'streamer-name' with the streamer, most likely your own
oauth       g0Bble0dEE0GukK0enCryPTIon0KEy // <- With or without "oauth:" prefix
```

エモジ看板 ( エモジかんばん | emoji kanban | Emoji/Emote Signboard )

