use super::Lobby;
use super::{LobbyChat, Player, PlayerKill};
use crate::tf2::lobby::AccountAge;
use crate::tf2::rcon::{G15DumpPlayerOutput, G15PlayerData};
use crate::tf2::steamapi::SteamApiMsg;
use crate::tf2bd::Tf2bdMsg;
use crate::{
    appbus::AppBus,
    models::{app_settings::AppSettings, steamid::SteamID},
    tf2::logfile::LogLine,
};
use bus::BusReader;
use chrono::prelude::*;
use std::collections::HashSet;
use std::{
    sync::{Arc, Mutex},
    thread::{self, sleep},
};

/// The delay between loops in run()
const LOOP_DELAY: std::time::Duration = std::time::Duration::from_millis(500);

/// The number of seconds a player can be in the recently_left_players collection
const RECENTLY_LEFT_TIMEOUT_REMOVAL_SECONDS: i64 = 90;

pub struct LobbyThread {
    bus: Arc<Mutex<AppBus>>,
    logfile_bus_rx: BusReader<LogLine>,
    steamapi_bus_rx: BusReader<SteamApiMsg>,
    tf2bd_bus_rx: BusReader<Tf2bdMsg>,
    g15_bus_rx: BusReader<G15DumpPlayerOutput>,
    lobby: Lobby,
}

/// Start the background thread for the lobby module
pub fn start(settings: &AppSettings, bus: &Arc<Mutex<AppBus>>) -> thread::JoinHandle<()> {
    let mut lobby_thread = LobbyThread::new(settings, bus);

    thread::spawn(move || lobby_thread.run())
}

impl LobbyThread {
    pub fn new(settings: &AppSettings, bus: &Arc<Mutex<AppBus>>) -> Self {
        let logfile_bus_rx = bus.lock().unwrap().logfile_bus.add_rx();
        let steamapi_bus_rx = bus.lock().unwrap().steamapi_bus.add_rx();
        let tf2bd_bus_rx = bus.lock().unwrap().tf2bd_bus.add_rx();
        let g15_bus_rx = bus.lock().unwrap().g15_report_bus.add_rx();

        Self {
            bus: Arc::clone(bus),
            logfile_bus_rx,
            steamapi_bus_rx,
            tf2bd_bus_rx,
            g15_bus_rx,
            lobby: Lobby::new(settings.self_steamid64),
        }
    }

    pub fn run(&mut self) {
        log::info!("Lobby background thread started");

        loop {
            self.process_bus();
            self.lobby.update_friendships();
            self.update_scoreboard();

            sleep(LOOP_DELAY);
        }
    }

    fn process_bus(&mut self) {
        self.purge_old_players();
        self.process_g15_bus();
        self.process_logfile_bus();
        self.process_steamapi_bus();
        self.process_tf2bd_bus();
    }

    fn process_g15_bus(&mut self) {
        while let Ok(g15_dump) = self.g15_bus_rx.try_recv() {
            self.process_g15_dump(g15_dump);
        }
    }

    fn process_g15_dump(&mut self, g15_dump: G15DumpPlayerOutput) {
        let now = Local::now();

        // Merge data from the G15 dump into the lobby
        for player in g15_dump.players.iter() {
            if let Some(lobby_player) = self.lobby.get_player_mut(None, Some(player.steamid)) {
                // Player already exists in the lobby
                Self::merge_player_g15_data(lobby_player, player);
                lobby_player.last_seen = now;
            } else {
                // Player is new, add it to the lobby
                log::info!("Player {} has joined", player.name.clone().to_string());
                let mut lobby_player = Player::default();
                Self::merge_player_g15_data(&mut lobby_player, player);
                lobby_player.last_seen = now;

                self.lobby.players.push(lobby_player);
            };
        }

        // Remove players from the lobby that are not in the G15 dump
        let g15_steamids: HashSet<SteamID> = g15_dump.players.iter().map(|p| p.steamid).collect();
        let mut players_to_keep: Vec<Player> = vec![];
        for player in self.lobby.players.iter() {
            if g15_steamids.contains(&player.steamid) {
                players_to_keep.push(player.clone());
            } else {
                log::info!("Player {} has left", player.name);
                self.lobby.recently_left_players.push(player.clone());
            }
        }
        self.lobby.players = players_to_keep;
    }

    fn merge_player_g15_data(lobby_player: &mut Player, player: &G15PlayerData) {
        lobby_player.steamid = player.steamid;

        lobby_player.id = player.id;
        lobby_player.name = player.name.clone();
        if player.team.is_some() {
            lobby_player.team = player.team.unwrap();
        }
        lobby_player.alive = player.alive;
        lobby_player.pingms = player.ping;
        lobby_player.score = player.score;
    }

    fn process_tf2bd_bus(&mut self) {
        while let Ok(msg) = self.tf2bd_bus_rx.try_recv() {
            match msg {
                Tf2bdMsg::Tf2bdPlayerMarking(steamid, player_info) => {
                    if let Some(player) = self.lobby.get_player_mut(None, Some(steamid)) {
                        player.player_info = player_info;
                    }
                }
            }
        }
    }

    fn process_steamapi_bus(&mut self) {
        while let Ok(msg) = self.steamapi_bus_rx.try_recv() {
            match msg {
                SteamApiMsg::FriendsList(steamid, friends) => {
                    if let Some(player) = self.lobby.get_player_mut(None, Some(steamid)) {
                        player.friends = Some(friends);
                    }
                }
                SteamApiMsg::PlayerSummary(player_steam_info) => {
                    if let Some(player) = self
                        .lobby
                        .get_player_mut(None, Some(player_steam_info.steamid))
                    {
                        match player_steam_info.account_age {
                            Some(account_age) => {
                                player.account_age = AccountAge::Loaded(account_age);
                            }
                            None => {
                                player.account_age = AccountAge::Private;
                            }
                        }
                        player.steam_info = Some(player_steam_info);
                    }
                }
                SteamApiMsg::Tf2Playtime(steamid, playtime) => {
                    if let Some(player) = self.lobby.get_player_mut(None, Some(steamid)) {
                        player.tf2_play_minutes = playtime;
                    }
                }
                SteamApiMsg::SteamBans(steamid, steam_bans) => {
                    if let Some(player) = self.lobby.get_player_mut(None, Some(steamid)) {
                        player.steam_bans = Some(steam_bans);
                    }
                }
                SteamApiMsg::ApproxAccountAge(steamid, account_age) => {
                    if let Some(player) = self.lobby.get_player_mut(None, Some(steamid)) {
                        player.account_age = account_age;
                    }
                }
            }
        }
    }

    fn process_logfile_bus(&mut self) {
        while let Ok(cmd) = self.logfile_bus_rx.try_recv() {
            match cmd {
                LogLine::Kill {
                    when,
                    killer,
                    victim,
                    weapon,
                    crit,
                } => self.kill(when, killer, victim, weapon, crit),
                LogLine::Suicide { when, name } => self.suicide(when, name),
                LogLine::LobbyCreated { when } => self.new_lobby(when),
                LogLine::LobbyDestroyed { when: _when } => {}
                LogLine::Chat {
                    when,
                    name,
                    message,
                    dead,
                    team,
                } => self.chat(when, name, message, dead, team),
            }
        }
    }

    fn update_scoreboard(&mut self) {
        let mut bus = self.bus.lock().unwrap();
        bus.send_lobby_report(self.lobby.clone());
    }

    fn new_lobby(&mut self, when: DateTime<Local>) {
        log::info!("*** Creating new lobby ***");

        for player in self.lobby.players.iter_mut() {
            player.last_seen = when;
        }

        self.lobby
            .recently_left_players
            .append(&mut self.lobby.players);

        log::info!(
            "Moving players to recently_left_players: {}",
            self.lobby
                .recently_left_players
                .iter()
                .map(|p| p.name.clone())
                .collect::<Vec<String>>()
                .join(", ")
        );

        self.lobby.lobby_id = Local::now().format("%Y-%m-%d").to_string();

        self.lobby.players.clear();
        self.lobby.chat.clear();
    }

    fn kill(
        &mut self,
        _when: DateTime<Local>,
        killer: String,
        victim: String,
        weapon: String,
        crit: bool,
    ) {
        // log::info!("Kill: {} killed {} with {}", killer, victim, weapon);
        if let Some(player) = self.lobby.get_player_mut(Some(killer.as_str()), None) {
            player.kills += 1;
            if crit {
                player.crit_kills += 1;
            }
            player.kills_with.push(PlayerKill {
                weapon: weapon.clone(),
                crit,
            });
        } else {
            log::warn!("Killer not found: '{}'", victim);
        }

        if let Some(player) = self.lobby.get_player_mut(Some(victim.as_str()), None) {
            player.deaths += 1;
            if crit {
                player.crit_deaths += 1;
            }
        } else {
            log::warn!("Victim not found: '{}'", victim);
        }
    }

    fn suicide(&mut self, _when: DateTime<Local>, name: String) {
        if let Some(player) = self.lobby.get_player_mut(Some(name.as_str()), None) {
            player.deaths += 1;
        } else {
            log::warn!("Player not found: '{}'", name);
        }
    }

    fn chat(
        &mut self,
        when: DateTime<Local>,
        name: String,
        message: String,
        dead: bool,
        team: bool,
    ) {
        if let Some(player) = self.lobby.get_player(Some(name.as_str()), None) {
            self.lobby.chat.push(LobbyChat {
                when,
                steamid: player.steamid,
                player_name: name,
                message,
                dead,
                team,
            })
        } else {
            log::warn!("Player not found: '{}'", name);
        }
    }

    // Go through the recently_left_players
    // and remove those who are still active
    // and remove those who are older than a certain seconds
    fn purge_old_players(&mut self) {
        let when = Local::now();

        let lobby_steamids: HashSet<SteamID> =
            self.lobby.players.iter().map(|p| p.steamid).collect();

        let mut recently_left_to_keep: Vec<Player> = vec![];
        for player in self.lobby.recently_left_players.iter() {
            if lobby_steamids.contains(&player.steamid) {
                // The player also exists in the active player list
                log::info!(
                    "Player {} has returned..............................",
                    player.name
                );
                continue;
            }

            let age = when - player.last_seen;
            if age.num_seconds() < RECENTLY_LEFT_TIMEOUT_REMOVAL_SECONDS {
                // Keep the player in the recently list for a bit longer
                recently_left_to_keep.push(player.clone());
            } else {
                log::info!("Player {} is being deleted from recently list", player.name);
            }
        }

        self.lobby.recently_left_players = recently_left_to_keep;
    }
}
