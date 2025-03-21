# TF2Monitor

[![Discord](https://gist.github.com/cxmeel/0dbc95191f239b631c3874f4ccf114e2/raw/discord.svg)](https://discord.gg/z9Cn7m3gdN)

**NOTE! This project is a work in progress and something I do on a hobby basis when I have spare time and energy.**

**NOTE! Not ready to receive code contributions yet.**

An application companion to be run along with you playing Team Fortress 2.

With this app you can:

- Collect and present public Steam info about the players you play with. Example info:

  - Steam account creating time
  - Player's public avatar picture - Buttons with links to popular sites such as SteamHistory.net and SteamCommunity.com for each player
  - Number of TF2 hours
  - VAC bans etc

- Get the chat in TF2 translated into English

Players marked are saved in playerlist.json. The format is based on TF2BD file format but only the SteamID -> flags section are used, and flags for Cool(one-way soft-friendship) and Bot are added.

# Screenshot

![Alt text](/screenshots/TF2Monitor.png?raw=true "TF2Monitor")

## Scoreboard

First column in the table is:

- Red box is Steam bans. Hover this box or the player for specifics.
- White box is you(settings file, self_steamid64)
- Green box is for accounts newer than one year
- Team colored box
- Heart is a friend of yours(settings file, self_steamid64)

Hover on a player name to see some brief information about the player. Players marked yellow are friends. Click on the player's name to see more details in the bottom-right panel.

Under scoreboard there's a list of players that left recently. Players remain there for 1 minute. Click on a player name to see details.

When "Show friendships" is checked, there are white connections between friends.

## Chat

Translated chat texts have black background and white text.

# What about VAC?

This application does not alter any game file or intervene with the TF2 process and memory in any way. This is completely VAC safe.

All it does is to start TF2 with some flags to tell it to log more info to the `console.log` file, and enable and configure RCON(remote connect) so this app can send commands to TF2. Then the application periodically scans the `console.log` file and sends commands to TF2 over RCON telling it to log info about the players in the server, to votekick cheaters and bots.

# Your privacy

Good that you reflected on this! This is after all an application downloaded from the internet, made by someone you probably don't know.

I have no intention nor interest in collecting any of your personal information or any identifiable data.

The source code is available for anyone to inspect.

# How to set up and run

At the moment you need to download the source code and [install The Rust programming language](https://www.rust-lang.org/tools/install).

After that you go to the folder where this `README.md` file is located with a command line/terminal prompt and type `cargo run` and the application will be compiled and started.

1. Start Steam. If Steam is running your SteamID is read from the Windows registry if you are on Windows
2. First start will complain about a missing `settings.json` file, and a skeleton settings file was created
3. Quit the app and open that `settings.json` in a text editor
4. Fill in your own SteamID, if you are on Windows and Steam was running, it is filled in already.
5. Fill in the SteamAPI key, go to https://steamcommunity.com/dev/apikey to create a personal one
6. `cargo run` again
7. Use a second terminal and run `start_tf2.bat`

## Start TF2 from the app or from Steam?

Use the `start_tf2.bat` for now.

If you change rcon-password or port you need to alter the bat file.

`start_tf2.bat`

```
"C:\Program Files (x86)\Steam\steamapps\common\Team Fortress 2\tf_win64.exe" -steam -game tf  -usercon -high +developer 1 +contimes 0 +ip 0.0.0.0 +sv_rcon_whitelist_address 127.0.0.1 +sv_quota_stringcmdspersecond 1000000 +rcon_password rconpwd +hostport 40434 +net_start +con_timestamp 1 -condebug -conclearlog -novid -nojoy -nosteamcontroller -nohltv -particles 1 -console
```

# TF2 Bot Detector files

This app can read and write the usual TF2 Bot Detector files but will only use the subset of rules that point out a SteamID and a flag(Cheater, Suspicious, Racist, Exploiter).

Where the `settings.json` file are your markings will be saved in `playerlist.json` whenever a player's flags are changed.

Right now only one `playerlist.json` file is supported.

In the future there could be support for multiple `playerlist.XXX.json` along with autoupdate features.

# Linux support?

I have not tested it but currently the app does not use any platform specifc API.

The UI framework uses OpenGL via `glow` and there's a note here on libs you might need to install on your Linux machine:
https://crates.io/crates/eframe/0.27.2

# Application architecture and technology

See the file [ARCHITECTURE.md](/ARCHITECTURE.md) for the bigger picture of the inner workings of this app.

The application is written in [Rust](https://www.rust-lang.org/) and uses [egui](https://www.egui.rs/) to display an user interface.

# Other projects of interest

There are already several similar applications like this one. Some of them has been tested for years and offers a lot of functionality.

- https://botdetector.tf/

  - Newly developed.
  - Uses a Go backend.
  - Uses a web frontend written in React.

- https://github.com/PazerOP/tf2_bot_detector

  - This is archived and not developed.
  - With some tweaks it still works fine.
  - Has a lot of features.
  - Written in C++ with a Dear Imgui frontend.
  - Windows only AFAIK.
  - There are also forks with fixes and updates:
    - https://github.com/surepy/tf2_bot_detector
