use super::Lobby;
use super::{LobbyChat, Player, PlayerKill, Team};
use crate::tf2::lobby::{AccountAge, Tf2PlayMinutes};
use crate::tf2::steamapi::SteamApiMsg;
use crate::tf2bd::Tf2bdMsg;
use crate::{
    appbus::AppBus,
    models::{app_settings::AppSettings, steamid::SteamID},
    tf2::logfile::LogLine,
};
use bus::BusReader;
use chrono::prelude::*;
use std::{
    sync::{Arc, Mutex},
    thread::{self, sleep},
};

/// The delay between loops in run()
const LOOP_DELAY: std::time::Duration = std::time::Duration::from_millis(250);

/// The number of seconds a player can be in the recently_left_players collection
const RECENTLY_LEFT_TIMEOUT_REMOVAL_SECONDS: i64 = 90;

pub struct LobbyThread {
    bus: Arc<Mutex<AppBus>>,
    logfile_bus_rx: BusReader<LogLine>,
    steamapi_bus_rx: BusReader<SteamApiMsg>,
    tf2bd_bus_rx: BusReader<Tf2bdMsg>,
    lobby: Lobby,

    self_steamid: SteamID,

    last_status_header: DateTime<Local>,
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
        Self {
            self_steamid: settings.self_steamid64,
            bus: Arc::clone(bus),
            logfile_bus_rx,
            steamapi_bus_rx,
            tf2bd_bus_rx,
            lobby: Lobby::new(settings.self_steamid64),
            last_status_header: Local::now(),
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
        self.process_logfile_bus();
        self.process_steamapi_bus();
        self.process_tf2bd_bus();
    }

    fn process_tf2bd_bus(&mut self) {
        log::debug!("Processing tf2bd bus");
        while let Ok(msg) = self.tf2bd_bus_rx.try_recv() {
            match msg {
                Tf2bdMsg::Tf2bdPlayerMarking(steamid, source, marking) => {
                    if let Some(player) = self.lobby.get_player_mut(None, Some(steamid)) {
                        if let Some(marking) = marking {
                            player.flags.insert(source.clone(), marking);
                        } else {
                            player.flags.remove(&source);
                        }
                    }
                }
            }
        }
    }

    fn process_steamapi_bus(&mut self) {
        log::debug!("Processing steamapi bus");
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
        log::debug!("Processing logfile bus");
        while let Ok(cmd) = self.logfile_bus_rx.try_recv() {
            match cmd {
                LogLine::StatusHeader { when } => self.purge_old_players(when),
                LogLine::StatusForPlayer {
                    when,
                    id,
                    name,
                    steam_id32,
                } => self.player_seen(when, id, name, steam_id32),
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
                LogLine::PlayerTeam { steam_id32, team } => self.assign_team(steam_id32, team),
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

        self.lobby.players.clear();
        self.lobby.chat.clear();
    }

    /// Add this player to the list of players if not already added
    fn player_seen(&mut self, when: DateTime<Local>, id: u32, name: String, steam_id32: String) {
        // log::info!("Player seen: {} ({})", name, steam_id32);
        let steamid = SteamID::from_steam_id32(steam_id32.as_str());

        if let Some(player) = self.lobby.get_player_mut(None, Some(steamid)) {
            // Update last_seen for existing player
            player.id = id;
            player.name.clone_from(&name);
            player.last_seen = when;
        } else {
            // Add new player if not found in the list
            self.lobby.players.push(Player {
                id,
                steamid,
                name: name.clone(),
                team: Team::Unknown,
                kills: 0,
                deaths: 0,
                crit_kills: 0,
                crit_deaths: 0,
                kills_with: Vec::new(),
                last_seen: when,
                steam_info: None,
                friends: None,
                tf2_play_minutes: Tf2PlayMinutes::Loading,
                steam_bans: None,
                account_age: AccountAge::Loading,
                flags: Default::default(),
            });
        }
    }

    fn assign_team(&mut self, steam_id32: String, team: String) {
        let steamid = SteamID::from_steam_id32(steam_id32.as_str());

        let team = match team.as_str() {
            "INVADERS" => Team::Invaders,
            "DEFENDERS" => Team::Defendes,
            "SPEC" => Team::Spec,
            _ => Team::Unknown,
        };

        if let Some(player) = self.lobby.get_player_mut(None, Some(steamid)) {
            player.team = team;
        } else {
            // Add new player if not found in the list
            // self.lobby.players.push(Player {
            //     id: 0,
            //     steamid,
            //     name: steam_id32.clone(),
            //     team,
            //     kills: 0,
            //     deaths: 0,
            //     crit_kills: 0,
            //     crit_deaths: 0,
            //     kills_with: Vec::new(),
            //     last_seen: Local::now(),
            //     steam_info: None,
            //     friends: None,
            //     tf2_play_minutes: None,
            //     steam_bans: None,
            //     flags: Default::default(),
            // });
        }
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

    /// Players who has a last_seen older than 15 seconds are removed from the lobby
    /// and instead added to the recently_left collection.
    /// Recently_left players remain there until 30 seconds has passed.
    fn purge_old_players(&mut self, when: DateTime<Local>) {
        let mut players_to_keep: Vec<Player> = vec![];
        for player in self.lobby.players.iter_mut() {
            if player.last_seen >= self.last_status_header {
                // Player is still active, keep it
                players_to_keep.push(player.clone());
            } else {
                // Player has left the game
                // Add to recently_left_players
                // and update last_seen so it remains in the list for a while

                log::info!(
                    "Player left: {}. Changing last_seen from {} to {}",
                    player.name,
                    player.last_seen,
                    when
                );
                player.last_seen = when;
                self.lobby.recently_left_players.push(player.clone());
            }
        }

        self.lobby.players = players_to_keep;

        // Go through the recently_left_players
        // and remove those who are still active
        // and remove those who are older than a certain seconds
        let mut recently_left_to_keep: Vec<Player> = vec![];
        for player in self.lobby.recently_left_players.iter() {
            if self
                .lobby
                .players
                .iter()
                .any(|p| p.steamid == player.steamid)
            {
                // The player also exists in the active player list
                log::info!("Player {} has returned", player.name);
                continue;
            }

            let age = when - player.last_seen;
            if age.num_seconds() < RECENTLY_LEFT_TIMEOUT_REMOVAL_SECONDS {
                recently_left_to_keep.push(player.clone());
            } else {
                log::info!("Player {} has left for good", player.name);
            }
        }

        self.lobby.recently_left_players = recently_left_to_keep;

        self.last_status_header = when;
    }
}
