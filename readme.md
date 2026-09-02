 エモジ看板 emojiKanban
========================

OBS plugin Emote Wall 
---------------------

![emojikanban demo](emojikanban_demo.gif)

A local emote wall written in Rust as an OBS Plugin. No HTML, No JavaScript, No Meta Cookies. The only external reliance is on Twitch.tv (not that I've read the code of any of the libraries propping this creation up). A step towards self hosting whatever possible on open source software.

Plugin connects to Twitch via IRC and provides new Sources `EmojiKanBan`, an emote wall, and `ChattoKanBan`, a resizable chat window to remove the need for a Browser Source.  The emote wall monitors chat for emotes to be drawn to the screen with one of a few random effects. The maximum number of simultaneous emotes can be set in `Properties`. If the queue is at the limit, further emotes are ignored/skipped until there's room in the queue again. More emote wall control in the Properties dialog is planned.

About The Name
--------------

エモジ看板 ( エモジかんばん | emoji kanban | Emoji/Emote Signboard )

Status:
=======

Version 0.3.0 as of 2026-09-02 -> Now with built in Twitch Chat support, no more Browser Source nonsense. (Choice of colors used planned for 0.3.1)

Version 0.2.0 as of 2026-08-14 -> Now with FlatPak support!

Can be connected to Twitch from Properties menu!

I've written a lot of things in this readme.md and I've tried to keep it up to date, but no promises.

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
- [x] Support KDL for configuration
   - [ ] Add YouTube auth stuffs
   - [ ] Support the above planned Anti-Spam features
- [ ] Add YouTube live chat support

__Now cross-platform!__ (Minimal testing on Windows. Using `platform_dirs` crate. Should *just work*...)

Emotes are cached in a local sqlite database located in `[*nix: ~/.local/share | flatpak: ~/.var/app/com.obsproject.Studio/data | win: %LOCALAPPDATA% ]/emojikanban/emotes.db3` (untested on Windows) so that they are only downloaded once.

Use at your own risk :) Rust does not prevent errors in logic. The crate I use to make this an OBS plugin is **Archived** since 2025 which has been a mild problem. So I forked it and added a few things. Like buttons in the Properties window.

Basic Instructions:
===================

- After installing the plugin and ensuring that it's enabled, add `EmojiKanBan` and/or `ChattoKanBan` as a Source/Sources in your active Scene.
- Connect to Twitch:
  - In OBS Studio, select the Tools menu and click `EmojiKanBan Configuration`.
    - A Properties dialog will open in which you can enter:
      - Your Twitch bot account name (or your primary account)
      - The account name of the channel to be polled for chat
    - Once those values are correct, click `Apply Below Bot Account and Channel Values To Config`
  - Click `Request New Twitch OAuth Token And Connect` button.
    - After clicking the button, `http://localhost:3000` should open in your default web browser.
    - You should see "Use this **link** to authorize EmojiKanBan with Twitch"
      - The link is the same as the link in the **Need OAUTH?** section below.
    - If you're using a bot account, you may want to open the link in a Private Window.
      - However, it doesn't seem to matter whether I auth with my primary or bot account no matter which name I put in the 'bot account' field.
      - It may make a difference in the future if I add bot behaviour such as automated messages to chat
    - The server on `localhost:3000` will:
      - capture the authorization when complete
      - write the new OAuth token to EmojiKanBan's config file automatically
      - its duty fulfilled, the server will close

Details about the config file for manual editing:
  - EmojiKanBan will generate a configuration file if it does not already exist and initializes it with dummy data to be replaced with your `oauth` credentials
    - `[*nix: ~/.config | flatpak: ~/.var/app/com.obsproject.Studio/config | win: %APPDATA% ]/emojikanban/config.kdl`
      - Note: The file is now parsed as [KDL](https://kdl.dev/), but removing a key or value still may cause a failure to parse. [WIP]
      - Probably **DON'T** edit with `notepad.exe` as it messes with line endings. Notepad++ or a code editor is recommended.
        - This might be fixed... Untested
    - After `bot-account` change `bot-name` to your bot or streamer account name
    - After `channel` change `streamer-name` to the account you intend to monitor via IRC for emote usage (generally your streamer account)
      - If you remove or comment out the `channel` line, it will default to
    - The `oauth` line is now best handled within OBS in the emojikanban `Properties` window, though it can be acquired manually.
      - Instructions for manually acquiring the needed OAUTH token can be found below under **Need OAUTH?** section

Config
======

The config.kdl file: `[*nix: ~/.config | flatpak: ~/.var/app/com.obsproject.Studio/config | win: %APPDATA% ]/emojikanban/config.kdl`:
```kdl
bot-account bot-name                       // <- Replace 'bot-name' with the name of the account used to monitor chat
channel     streamer-name                  // <- and 'streamer-name' with the streamer, most likely your own
oauth       g0Bble0dEE0GukK0enCryPTIon0KEy // <- With or without "oauth:" prefix
```

Compilation/Installation
========================

Build on Linux:
```bash
git clone https://github.com/raum-dellamorte/emojikanban.git
cd emojikanban
./lin_build.sh
```

Linux Installation Copy/Pasta:
- built via `zigbuild_lin.sh` or `cargo zigbuild -r --target x86_64-unknown-linux-gnu.2.31` for Flatpak compatibility:
  - Flatpak OBS:
    - 1st:     `mkdir -p ~/.var/app/com.obsproject.Studio/config/obs-studio/plugins/emojikanban/bin/64bit/`
    - symlink: `ln -s $(pwd)/target/x86_64-unknown-linux-gnu/release/libemojikanban.so ~/.var/app/com.obsproject.Studio/config/obs-studio/plugins/emojikanban/bin/64bit/`
    - copy:    `cp $(pwd)/target/x86_64-unknown-linux-gnu/release/libemojikanban.so ~/.var/app/com.obsproject.Studio/config/obs-studio/plugins/emojikanban/bin/64bit/`
  - Native OBS:
    - symlink: `sudo ln -s $(pwd)/target/x86_64-unknown-linux-gnu/release/libemojikanban.so /usr/lib/obs-plugins/`
    - copy:    `sudo cp $(pwd)/target/x86_64-unknown-linux-gnu/release/libemojikanban.so /usr/lib/obs-plugins/`
- built via `cargo build -r`, substitute `$(pwd)/target/x86_64-unknown-linux-gnu/release/libemojikanban.so` with `$(pwd)/target/release/libemojikanban.so`

Build on Windows (requires Visual Studio Build Tools 2022 and LLVM):
```bash
git clone https://github.com/raum-dellamorte/emojikanban.git
cd emojikanban
.\win_build.bat
```

Windows Installation:
- read (or TrustMeBro) and then run `win_install.bat` with or without building first.
- or install manually:
  - in `C:\ProgramData\obs-studio\plugins\` create `emojikanban\bin\64bit\` if it doesn't exist.
    - if in portable mode, copy DLL into `obs-studio\obs-plugins\64bit\` directly.
    - if you installed OBS with scoop and want out of portable mode, delete `scoop\apps\obs-studio\current\portable_mode.txt`
  - then, into `C:\ProgramData\obs-studio\plugins\emojikanban\bin\64bit\`, copy whichever of the following applies:
    - without building:
      - `./bin/emojikanban.dll`
    - built via `win_build.bat` or `cargo build -r --target x86_64-pc-windows-msvc`:
      - `.\target\x86_64-pc-windows-msvc\release\emojikanban.dll`
    - built via `cargo build -r`:
      - `.\target\release\emojikanban.dll`

Feel free to submit a bug report if these instructions are wrong.

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

Authorization has been simplified!
See **Basic Instructions** above.

[Authorize emojiKanban](https://id.twitch.tv/oauth2/authorize?response_type=token&client_id=m0kk7y5gjs9qjfio2pw7hkw8iwaeft&redirect_uri=http://localhost:3000&scope=chat%3Aedit%20chat%3Aread)

Manual OAuth token aquisition:
- Link: 
- Open the link and sign in with your bot account, or streamer account if you want.

- After you click Authorize, you're automatically redirected to a localhost address that doesn't exist. In the URL bar you'll see:
  - `http://localhost:3000/#access_token=(this is your oauth token)&scope=chat%3Aedit+chat%3Aread&token_type=bearer`
  - The part between `...access_token=` and `&scope=...`  is your oauth token. Copy that into the config.kdl replacing `g0Bble0dEE0GukK0enCryPTIon0KEy`
  - Don't forget to replace `bot-name` and `streamer-name` appropriately. If using your streamer account as your bot account replace both with the streamer account name.

The Generating-Your-Own-Private-App Method:
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


