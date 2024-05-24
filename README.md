# TF2Monitor

An application companion to be run along with you playing Team Fortress 2.

With this app you can:

- Collect and present public Steam info about the players you play with. Example info:

  - Steam account creating time
  - Player's public avatar picture - Buttons with links to popular sites such as SteamHistory.net and SteamCommunity.com for each player.
  - TODO: Number of TF2 hours
  - TODO: VAC bans etc.

- TODO: Keep track of people you know are bots, cheaters, racists and so on.

**NOTE! This project is a work in progress and something I do on a hobby basis when I have spare time and energy.**

# Screenshot

![Alt text](/screenshots/TF2Monitor.png?raw=true "TF2Monitor")

First column in the table is:

- Red box is Steam bans. Hover this box or the player for specifics.
- White box is you(settings file, self_steamid64)
- Green box is for accounts newer than one year
- Team colored box
- Heart is a friend of yours(settings file, self_steamid64)

Hover on a player to show its friends. Players marked yellow are friends. Tooltip for a player also shows the friend's names.

Under scoreboard there's a list of players that left recently. Names are links to SteamHistory for now. They remain there for 1 minute.

When "Show friendships" is checked, there are white connections between friends.

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

# Linux support?

I have not tested it but currently the app does not use any platform specifc API.

The UI framework uses OpenGL via `glow` and there's a note here on libs you might need to install on your Linux machine:
https://crates.io/crates/eframe/0.27.2

# Other projects of interest

There are already several similar applications like this one. Most of them have been tested for years and offer more functionality.

- https://botdetector.tf/

  - Newly developed

- https://github.com/PazerOP/tf2_bot_detector
  - This is archived and not developed.
  - With some tweaks it still works fine.
  - There are also forks with fixes, if you search github.
