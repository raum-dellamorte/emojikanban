 エモジ看板 emojiKanban
========================

OBS plugin Emote Wall 
---------------------

![emojikanban demo](emojikanban_demo.gif)

A local emote wall written in Rust as an OBS Plugin. No HTML, No JavaScript, No Meta Cookies. The only external reliance is on Twitch.tv (not that I've read the code of any of the libraries propping this creation up). A step towards self hosting whatever possible on open source software.

Plugin gives new Source `emojikanban` which connects to Twitch via IRC and monitors chat for emotes to be drawn with some effect to the screen. It tries to make an effect with any emote used in chat (unless it's a single emote by itself for some reason). The maximum number of simultaneous emotes can be set in `Properties`. If the queue is at the limit, further emotes are ignored/skipped until there's room in the queue again.

About The Name
--------------

エモジ看板 ( エモジかんばん | emoji kanban | Emoji/Emote Signboard )

Status:
=======

__Effects:__
- [x] Gravity: 70% : Emote spawns at the top of the screen, falls, and bounces with a life between 2 and 5 seconds.
- [x] InchWorm: 20% : Emote spawns at the center as 9 segments that move in a random direction in an inchworm inspired fashion till offscreen.
- [x] SlideUp: 10% : Emote is scaled up to 512x, slides up from the bottom of the screen, pauses, and slides back down using smootherstep.
- [ ] JumpingPlumber: ??% : Legally Distinct Plumber jumps on emote as it slides along the bottom of the screen causing it to arc up a bit then fall offscreen. [Not Started]
- [ ] GrowingTree: ??% : Grow a tree of the emote, larger at the base and smaller toward the branches, then grow a fruit and drop it. Maybe too ambitious?

__Planned features__ other than effects:
- [ ] Spam Prevention/Mitigation features!
   - [ ] Configurable per effect activation percentages
   - [ ] Configurable global cooldowns, all effects or per effect
   - [ ] Configurable user cooldowns
   - [ ] Optionally require bits, points, follow or sub to activate for some amount of time
- [ ] Properly support KDL for configuration
   - [ ] Support the above planned Anti-Spam features

__Now cross-platform!__ (Minimal testing on Windows. Using `platform_dirs` crate. Should *just work*...)

Emotes are cached in a local sqlite database located in `[*nix: ~/.config | win: %APPDATA% ]/emojikanban/emotes.db3` (untested on Windows) so that they are only downloaded once.

Use at your own risk :) Rust does not prevent errors in logic. The crate I use to make this an OBS plugin is **Archived** since 2025 which may prove to be a problem in the near future.

Basic Instructions:
===================

- Add `emojikanban` as a source in your active scene after installing the plugin and ensuring that it's enabled. It generates a configuration file if it does not already exist and initializes it with dummy data to be replaced with your `oauth` credentials
  - `[*nix: ~/.config | win: %APPDATA% ]/emojikanban/config.kdl`
  - **DON'T** edit with `notepad.exe` as it may cause a failure to parse the file. Notepad++ or a code editor is recommended.
  - The plugin needs three strings from the user:
    - the `bot-account` (or streamer account) used to generate
    - the chat `channel` you intend to monitor via IRC for emote usage (generally your streamer account)
    - and the `oauth` token
  - The file is technically valid [KDL](https://kdl.dev/), but it isn't parsed as KDL, so don't change the order or remove a key or value.
  - These are stored as key value pairs parsed in fixed order with `nom` ~~for fun and profit~~.
  - It will be parsed as actual KDL in the future in order to handle more settings like the planned features above
- Instructions for acquiring the needed OAUTH token can be found below under "Need OAUTH?" heading
  - If you got the token with your streamer account, use your streamer account for both `bot-account` and `channel` in `config.kdl`
- Once `config.kdl` is filled out and saved, restart OBS Studio and it should connect to the IRC channel of your streamer chat
- Single emotes with no text don't yet work so test either with multiple emotes in a line or an emote with some text. The more emotes, the more fun!

Config
======

After first run, edit `[*nix: ~/.config | win: %APPDATA% ]/emojikanban/config.kdl`:
```kdl
bot-account bot-name                       // <- Replace 'bot-name' with the name of the account used to monitor chat
channel     streamer-name                  // <- and 'streamer-name' with the streamer, most likely your own
oauth       g0Bble0dEE0GukK0enCryPTIon0KEy // <- With or without "oauth:" prefix
```

Compilation/Installation
========================

```bash
git clone https://github.com/raum-dellamorte/emojikanban.git
cd emojikanban
cargo build -r
```

Linux installation should be either `sudo ln -s $(pwd)/target/release/libemojikanban.so /usr/lib/obs-plugins/` for ease of updating or `sudo cp $(pwd)/target/release/libemojikanban.so /usr/lib/obs-plugins/` if you don't intend to keep this repo after installation. The Windows DLL is at `target/x86_64-pc-windows-msvc/release/emojikanban.dll` after cross compiling but may be at `target/release/emojikanban.dll` if compiled on Windows with msvc Rust. Feel free to submit a bug report if these instructions are wrong.

Cross-compile from Linux to Windows:
```bash
git clone https://github.com/raum-dellamorte/emojikanban.git
cd emojikanban
rustup target add x86_64-pc-windows-msvc
cargo install cargo-xwin
cargo xwin build -r --target x86_64-pc-windows-msvc
```

I have not tried compiling on Windows. I've incuded `deps/obs.lib` generated from `obs.dll` from the 32.0.4 Windows release of OBS-Studio in order to compile the project for the `x86_64-pc-windows-msvc` target. As long as you're using the msvc version of Rust, it should compile like normal with `cargo build -r` on Windows.

Generating `obs.lib` on Arch:
```bash
yay -S llvm mingw-w64-tools
gendef obs.dll
llvm-dlltool -m i386:x86-64 -d obs.def -l obs.lib
```

Need OAUTH?
===========

The ~~TrustMeBro~~ Easy Method:
-------------------------------

[Authorize emojiKanban](https://id.twitch.tv/oauth2/authorize?response_type=token&client_id=m0kk7y5gjs9qjfio2pw7hkw8iwaeft&redirect_uri=http://localhost:3000&scope=chat%3Aedit%20chat%3Aread)

- Open the link and sign in with your bot account, or streamer account if you want.

- After you click Authorize, you're automatically redirected to a localhost address that doesn't exist. In the URL bar you'll see:
  - `http://localhost:3000/#access_token=(this is your oauth token)&scope=chat%3Aedit+chat%3Aread&token_type=bearer`
  - The part between `...access_token=` and `&scope=...`  is your oauth token. Copy that into the config.kdl replacing `g0Bble0dEE0GukK0enCryPTIon0KEy`
  - Don't forget to replace `bot-name` and `streamer-name` appropriately. If using your streamer account as your bot account replace both with the streamer account name.

Generating Your Own Private App Method:
---------------------------------------

This is how I made the ~~TrustMeBro~~ Link above.

At Your Own Risk, Do The Following:

Create Twitch Application:
- Log into `https://dev.twitch.tv/` with your bot account (or streamer account if you'd rather). __TFA__ must be enabled on that account.

- In the top right corner there should be a button that says "Your Console". Click it.

- On the left hand side you should see "Dashboard" "Extensions" "Applications" "Resources". Click "Applications".

- On the right near the top there will be a button that says "+ Register Your Application". Click it.

- The "Name" field has to be unique, so make up something you like. "emojiKanban" is taken. You can rename it or create a new App later. No pressure.

- For "OAuth Redirect URLs" copy and paste this: `http:\\localhost:3000`

- For "Category", best to pick "Chat Bot".

- "Client Type" defaults to "Confidential", leave it.

- Click "Create"

- You should now see what you just created in a list and there should be a button that says "Manage" by it. Click it.

- You should now have a "Client ID" at the bottom. 

Generate Auth URL with Client ID:
- I recommend opening a text editor to make the URL you need. Copy and paste this `https://id.twitch.tv/oauth2/authorize?response_type=token&client_id=`

- then copy and paste your Client ID immediately after the `=` with no spaces ...

- then, again with no spaces, copy and paste this after your Client ID `&redirect_uri=http://localhost:3000&scope=chat%3Aedit%20chat%3Aread`

  - `&scope=` is the permissions you're granting and they're set to `chat:edit chat:read` for IRC access.

  - You can add a `&state=PutRandomWordsHere` for security. See https://dev.twitch.tv/docs/authentication/getting-tokens-oauth/ for more information.

- Your URL should now look like this (of course with your own Client ID and, optionally, your own state string):
  - `https://id.twitch.tv/oauth2/authorize?response_type=token&client_id=ThisIsNotARealClientID&redirect_uri=http://localhost:3000&scope=chat%3Aedit%20chat%3Aread&state=OptionalStateForExample`

- Once you have your URL all put together, copy the whole thing and paste it into a new tab in the same browser where you logged into `dev.twitch.tv`. 

- You'll get an Authorize page with the name you chose for the Application you created. Click "Authorize".

- You'll get an `Unable to connect` page, but what you need is in the URL bar now. You should have been redirected to:
  - `http://localhost:3000/#access_token=(this is your oauth token)&scope=chat%3Aedit+chat%3Aread&state=OptionalStateForExample&token_type=bearer`

- Copy the part between `...access_token=` and `&scope=...` and that's your oauth token.


