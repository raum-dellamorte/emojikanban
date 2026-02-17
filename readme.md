 エモジ看板 emojiKanban
========================

OBS plugin Emote Wall 
---------------------

![emojikanban demo](emojikanban_demo.gif)

A local emote wall written in Rust as an OBS Plugin. No HTML, No JavaScript, No Meta Cookies. The only external reliance is on Twitch.tv (not that I've read the code of any of the libraries propping this creation up). A step towards self hosting whatever possible on open source software.

Plugin gives new Source `emojikanban` which connects to Twitch via IRC and monitors chat for emotes to be drawn with some effect to the screen. It tries to mak an effect with any emote used in chat (unless it's a single emote by itself for some reason). The maximum number of simultaneous emotes can be set in `Properties`. If the queue is at the limit, further emotes are ignored/skipped until there's room in the queue again.

There are currently two effects.
- Gravity: 70% : Emote spawns at the top of the screen, falls, and bounces with a life between 2 and 5 seconds.
- InchWorm: 20% : Emote spawns at the center as 9 segments that move in a random direction in an inchworm inspired fashion till offscreen.
- SlideUp: 10% : Emote is scaled up to 512x, slides up from the bottom of the screen, pauses, and slides back down using smootherstep.


On first use, generates `~/.config/emojikanban/config.kdl` with dummy data to be replaced with `oauth` credentials, and while I believe it's valid KDL, it isn't parsed as KDL, so don't change the order or remove a key or value. There may be room for improvement, but the plugin really just needs three strings from the user; the `bot-account` (or streamer account) used to generate the `oauth` token, the chat `channel` you intend to monitor via IRC for emote usage (generally your streamer account), and the `oauth` token. These are all stored as key value pairs in `config.kdl` which is parsed in fixed order with `nom` ~~for fun and profit~~. It may be parsed as actual KDL in the future, especially if I want to include more settings.

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

