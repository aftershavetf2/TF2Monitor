use super::Lobby;
use super::{LobbyChat, Player, PlayerKill, Team};
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
const LOOP_DELAY: std::time::Duration = std::time::Duration::from_millis(5000);

pub struct LobbyThread {
    bus: Arc<Mutex<AppBus>>,
    logfile_bus_rx: BusReader<LogLine>,
    lobby: Lobby,
}

/// Start the background thread for the lobby module
pub fn start(settings: &AppSettings, bus: &Arc<Mutex<AppBus>>) -> thread::JoinHandle<()> {
    let mut lobby_thread = LobbyThread::new(settings, bus);

    thread::spawn(move || lobby_thread.run())
}

impl LobbyThread {
    pub fn new(_settings: &AppSettings, bus: &Arc<Mutex<AppBus>>) -> Self {
        let logfile_bus_rx = bus.lock().unwrap().logfile_bus.add_rx();
        Self {
            bus: Arc::clone(bus),
            logfile_bus_rx,
            lobby: Lobby::new(),
        }
    }

    pub fn run(&mut self) {
        log::info!("Lobby background thread started");

        loop {
            self.process_bus();

            self.update_scoreboard();

            sleep(LOOP_DELAY);
        }
    }

    fn process_bus(&mut self) {
        while let Ok(cmd) = self.logfile_bus_rx.try_recv() {
            // log::info!("LobbyThread: Got message: {:?}", cmd);
            match cmd {
                LogLine::Unknown { line: _ } => {}
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
                LogLine::LobbyCreated { when: _when } => self.new_lobby(),
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

    fn new_lobby(&mut self) {
        log::info!("Creating new lobby");
        self.lobby = Lobby::new();
    }

    /// Add this player to the list of players if not already added
    fn player_seen(&mut self, when: DateTime<Local>, id: u32, name: String, steam_id32: String) {
        // log::info!("Player seen: {} ({})", name, steam_id32);
        let steamid = SteamID::from_steam_id32(steam_id32.as_str());

        // Update last_seen for existing player
        for player in self.lobby.players.iter_mut() {
            if player.steamid == steamid {
                player.name.clone_from(&name);
                player.last_seen = when;
                return;
            }
        }

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
        });
    }

    fn assign_team(&mut self, steam_id32: String, team: String) {
        let steamid = SteamID::from_steam_id32(steam_id32.as_str());

        for player in self.lobby.players.iter_mut() {
            if player.steamid == steamid {
                match team.as_str() {
                    "INVADERS" => player.team = Team::Invaders,
                    "DEFENDERS" => player.team = Team::Defendes,
                    "SPEC" => player.team = Team::Spec,
                    _ => player.team = Team::Unknown,
                }
                return;
            }
        }

        // Add new player if not found in the list
        self.lobby.players.push(Player {
            id: 0,
            steamid,
            name: steam_id32.clone(),
            team: Team::Unknown,
            kills: 0,
            deaths: 0,
            crit_kills: 0,
            crit_deaths: 0,
            kills_with: Vec::new(),
            last_seen: Local::now(),
        });
    }

    fn kill(
        &mut self,
        _when: DateTime<Local>,
        killer: String,
        victim: String,
        weapon: String,
        crit: bool,
    ) {
        if let Some(player) = self.lobby.get_player_by_name_mut(killer.as_str()) {
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

        if let Some(player) = self.lobby.get_player_by_name_mut(victim.as_str()) {
            player.deaths += 1;
            if crit {
                player.crit_deaths += 1;
            }
        } else {
            log::warn!("Victim not found: '{}'", victim);
        }
    }

    fn suicide(&mut self, _when: DateTime<Local>, name: String) {
        if let Some(player) = self.lobby.get_player_by_name_mut(name.as_str()) {
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
        if let Some(player) = self.lobby.get_player_by_name(name.as_str()) {
            self.lobby.chat.push(LobbyChat {
                when,
                steamid: player.steamid,
                message,
                dead,
                team,
            })
        } else {
            log::warn!("Player not found: '{}'", name);
        }
    }

    fn purge_old_players(&mut self, when: DateTime<Local>) {
        let mut new_vec: Vec<Player> = vec![];

        for player in self.lobby.players.iter_mut() {
            let age = when - player.last_seen;
            let seconds = age.num_seconds();
            if seconds < 30 {
                new_vec.push(player.clone());
            }
        }

        self.lobby.players = new_vec;
    }
}
