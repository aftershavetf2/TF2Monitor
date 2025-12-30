# TF2Monitor

[![Join the Discord](https://gist.github.com/cxmeel/0dbc95191f239b631c3874f4ccf114e2/raw/discord.svg)](https://discord.gg/z9Cn7m3gdN)

An application to be run along when you are playing Team Fortress 2.

- View public info about the players you play with:

  - Steam account creating time
  - Buttons with links to popular sites such as SteamHistory.net and SteamCommunity.com for each player
  - Number of TF2 hours
  - VAC and Game bans
  - Steam friends list
  - SourceBans reputation information

- Get the chat in TF2 translated into English

- Mark players with flags (Cheater, Bot, Suspicious, etc.) and save them to `playerlist.json`

- Automatically votekick cheaters and bots (configurable in settings)

Players marked are saved in playerlist.json. The format is based on TF2BD file format and Bot category has been added.

# Screenshot

![Alt text](/screenshots/TF2Monitor.png?raw=true "TF2Monitor")

# Getting Started

There are pre-compiled binaries you can use, but you can compile TF2Monitor yourself if you like. The pre-compiled binaries can be found here: https://github.com/aftershavetf2/TF2Monitor/releases

## Configuration

1. Start Steam. If Steam is running your SteamID is read from the Windows registry if you are on Windows. For Linux you have to manually set it in the settings.json after it has been created.

2. _IMPORTANT_: First start will complain about a missing `settings.json` file, and a skeleton settings file was created.

3. Quit TF2Monitor and open that `settings.json` in a text editor.

4. Fill in your own SteamID, if you are on Windows and Steam was running, it is filled in already.

5. Fill in the SteamAPI key, go to https://steamcommunity.com/dev/apikey to create a personal one. This is needed to fetch information from Steam Community. It is optional but nice.

6. Start TF2Monitor again

7. Use a second terminal and run `start_tf2.bat`

## How to Compile TF2Monitor

To compile it yourself you need to download the source code and [install The Rust programming language](https://www.rust-lang.org/tools/install).

After that you go to the folder where this `README.md` file is located with a command line/terminal prompt and type `cargo run --release` and the application will be compiled and started.

## Start TF2 from the app or from Steam?

Use the `start_tf2.bat` for now.

If you change rcon-password or port you need to alter the bat file.

`start_tf2.bat`

```
"C:\Program Files (x86)\Steam\steamapps\common\Team Fortress 2\tf_win64.exe" -steam -game tf -usercon -high +developer 1 +contimes 0 +ip 0.0.0.0 +sv_rcon_whitelist_address 127.0.0.1  +rcon_password rconpwd +hostport 40434 +net_start +con_timestamp 1 -condebug -conclearlog -novid -nojoy -nosteamcontroller -nohltv -particles 1 -console -g15
```

# Player Markings and Reputation

## TF2 Bot Detector files

This app can read and write the usual TF2 Bot Detector files but will only use the subset of rules that point out a SteamID and a flag(Cheater, Suspicious, Racist, Exploiter).

Where the `settings.json` file are your markings will be saved in `playerlist.json` whenever a player's flags are changed.

Right now only one `playerlist.json` file is supported.

## SourceBans Integration

The app integrates with SourceBans to fetch ban information for players. Players with bans from SourceBans will have their reputation marked accordingly.

# What about VAC?

This application does not:

- Alter any game file, or any other file at all

- Intervene with the TF2 process and memory in any way. No DLL injection or anything like that.

- It doesn't draw anything onto the TF2 screen

This is completely VAC safe.

All it does is:

- Start TF2 with some flags to tell it to log more info to the `console.log` file, and enable and configure RCON(remote connect) so this app can send commands to TF2.

- Then the application periodically reads from the `console.log` file

- Sends commands to TF2 over RCON telling it to log info about the players in the server, to votekick cheaters and bots.

# Privacy Statement

This application does not collect any personal information about you. It does not contain any analytics or anything like that.

It does fetch information from Steam Community and various community server sites, to fetch player account information and SourceBans about the SteamID's found in your current TF2 session..

The full source code is available for anyone to inspect.

# Linux support?

I have not tested it but currently the app does not use any platform specifc API. My intention is that it should work.

The UI framework uses OpenGL via `glow` and there's a note here on libs you might need to install on your Linux machine:
https://crates.io/crates/eframe/0.27.2

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
