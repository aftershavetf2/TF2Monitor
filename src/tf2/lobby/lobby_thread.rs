use super::shared_lobby::SharedLobby;
use super::LobbyKill;
use super::{LobbyChat, Player, PlayerKill};
use crate::config::LOBBY_LOOP_DELAY;
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
use translators::{GoogleTranslator, Translator};

/// The number of seconds a player can be in the recently_left_players collection
const RECENTLY_LEFT_TIMEOUT_REMOVAL_SECONDS: i64 = 90;

pub struct LobbyThread {
    logfile_bus_rx: BusReader<LogLine>,
    steamapi_bus_rx: BusReader<SteamApiMsg>,
    tf2bd_bus_rx: BusReader<Tf2bdMsg>,
    g15_bus_rx: BusReader<G15DumpPlayerOutput>,
    shared_lobby: SharedLobby,

    text_translator: GoogleTranslator,
}

/// Start the background thread for the lobby module
pub fn start(settings: &AppSettings, bus: &Arc<Mutex<AppBus>>) -> thread::JoinHandle<()> {
    let mut lobby_thread = LobbyThread::new(settings, bus);

    thread::spawn(move || lobby_thread.run())
}

impl LobbyThread {
    pub fn new(_settings: &AppSettings, bus: &Arc<Mutex<AppBus>>) -> Self {
        let logfile_bus_rx = bus.lock().unwrap().logfile_bus.add_rx();
        let steamapi_bus_rx = bus.lock().unwrap().steamapi_bus.add_rx();
        let tf2bd_bus_rx = bus.lock().unwrap().tf2bd_bus.add_rx();
        let g15_bus_rx = bus.lock().unwrap().g15_report_bus.add_rx();
        let shared_lobby = bus.lock().unwrap().shared_lobby.clone();

        let google_translator = GoogleTranslator::default();

        Self {
            logfile_bus_rx,
            steamapi_bus_rx,
            tf2bd_bus_rx,
            g15_bus_rx,
            shared_lobby,

            text_translator: google_translator,
        }
    }

    pub fn run(&mut self) {
        log::info!("Lobby background thread started");

        loop {
            self.process_bus();
            {
                let mut lobby = self.shared_lobby.get_mut();
                lobby.update_friendships();
            }
            self.translate_chat();

            sleep(LOBBY_LOOP_DELAY);
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
        let mut lobby = self.shared_lobby.get_mut();

        // Merge data from the G15 dump into the lobby
        for player in g15_dump.players.iter() {
            if let Some(lobby_player) = lobby.get_player_mut(None, Some(player.steamid)) {
                // Player already exists in the lobby
                Self::merge_player_g15_data(lobby_player, player);
                lobby_player.last_seen = now;
            } else {
                // Player is new, add it to the lobby
                log::info!("Player {} has joined", player.name.clone());
                let mut lobby_player = Player::default();
                Self::merge_player_g15_data(&mut lobby_player, player);
                lobby_player.last_seen = now;

                lobby.players.push(lobby_player);
            };
        }

        // Remove players from the lobby that are not in the G15 dump
        let g15_steamids: HashSet<SteamID> = g15_dump.players.iter().map(|p| p.steamid).collect();
        let mut players_to_keep: Vec<Player> = vec![];
        let mut players_to_move: Vec<Player> = vec![];
        for player in lobby.players.iter() {
            if g15_steamids.contains(&player.steamid) {
                players_to_keep.push(player.clone());
            } else {
                log::info!("Player {} has left", player.name);
                players_to_move.push(player.clone());
            }
        }
        lobby.players = players_to_keep;
        lobby.recently_left_players.append(&mut players_to_move);
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
        lobby_player.health = player.health;
    }

    fn process_tf2bd_bus(&mut self) {
        while let Ok(msg) = self.tf2bd_bus_rx.try_recv() {
            match msg {
                Tf2bdMsg::Tf2bdPlayerMarking(steamid, player_info) => {
                    self.shared_lobby.update_player(steamid, |player| {
                        player.player_info = player_info;
                    });
                }
            }
        }
    }

    fn process_steamapi_bus(&mut self) {
        while let Ok(msg) = self.steamapi_bus_rx.try_recv() {
            match msg {
                SteamApiMsg::FriendsList(steamid, friends) => {
                    self.shared_lobby.update_player(steamid, |player| {
                        player.friends = Some(friends);
                    });
                }
                SteamApiMsg::PlayerSummary(player_steam_info) => {
                    let steamid = player_steam_info.steamid;
                    let account_age = player_steam_info.account_age;
                    self.shared_lobby.update_player(steamid, |player| {
                        match account_age {
                            Some(age) => {
                                player.account_age = AccountAge::Loaded(age);
                            }
                            None => {
                                player.account_age = AccountAge::Private;
                            }
                        }
                        player.steam_info = Some(player_steam_info);
                    });
                }
                SteamApiMsg::Tf2Playtime(steamid, playtime) => {
                    self.shared_lobby.update_player(steamid, |player| {
                        player.tf2_play_minutes = playtime;
                    });
                }
                SteamApiMsg::SteamBans(steamid, steam_bans) => {
                    self.shared_lobby.update_player(steamid, |player| {
                        player.steam_bans = Some(steam_bans);
                    });
                }
                SteamApiMsg::ApproxAccountAge(steamid, account_age) => {
                    self.shared_lobby.update_player(steamid, |player| {
                        player.account_age = account_age;
                    });
                }
                SteamApiMsg::ProfileComments(steamid, comments) => {
                    self.shared_lobby.update_player(steamid, |player| {
                        player.profile_comments = Some(comments);
                    });
                }
                SteamApiMsg::Reputation(reputation) => {
                    let steamid = reputation.steamid;
                    self.shared_lobby.update_player(steamid, |player| {
                        player.reputation = Some(reputation);
                    });
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

    fn new_lobby(&mut self, when: DateTime<Local>) {
        let mut lobby = self.shared_lobby.get_mut();
        log::info!("*** Creating new lobby ***");

        for player in lobby.players.iter_mut() {
            player.last_seen = when;
        }

        let mut players_to_move = std::mem::take(&mut lobby.players);
        lobby.recently_left_players.append(&mut players_to_move);

        log::info!(
            "Moving players to recently_left_players: {}",
            lobby
                .recently_left_players
                .iter()
                .map(|p| p.name.clone())
                .collect::<Vec<String>>()
                .join(", ")
        );

        lobby.chat_msg_id = 0;
        lobby.lobby_id = Local::now().format("%Y-%m-%d").to_string();

        lobby.players.clear();
        lobby.chat.clear();
        lobby.kill_feed.clear();
    }

    fn kill(
        &mut self,
        when: DateTime<Local>,
        killer_name: String,
        victim_name: String,
        weapon: String,
        crit: bool,
    ) {
        let mut lobby = self.shared_lobby.get_mut();

        // Change the counts of the kill and death to the players
        if let Some(killer) = lobby.get_player_mut(Some(killer_name.as_str()), None) {
            killer.kills += 1;
            if crit {
                killer.crit_kills += 1;
            }
            killer.kills_with.push(PlayerKill {
                weapon: weapon.clone(),
                crit,
            });
        } else {
            log::warn!("Killer not found: '{}'", victim_name);
        }

        if let Some(victim) = lobby.get_player_mut(Some(victim_name.as_str()), None) {
            victim.deaths += 1;
            if crit {
                victim.crit_deaths += 1;
            }
        } else {
            log::warn!("Victim not found: '{}'", victim_name);
        }

        // Add the kill to the feed
        let (killer_steamid, victim_steamid) = {
            let killer = lobby.get_player(Some(killer_name.as_str()), None);
            let victim = lobby.get_player(Some(victim_name.as_str()), None);
            match (killer, victim) {
                (Some(k), Some(v)) => (Some(k.steamid), Some(v.steamid)),
                _ => {
                    log::info!(
                        "Killer or victim not found: '{}', '{}'",
                        killer_name,
                        victim_name
                    );
                    return;
                }
            }
        };

        if let (Some(killer_steamid), Some(victim_steamid)) = (killer_steamid, victim_steamid) {
            lobby.kill_feed.push(LobbyKill {
                when,
                killer: killer_steamid,
                victim: victim_steamid,
                weapon,
                crit,
            });
        }
    }

    fn suicide(&mut self, _when: DateTime<Local>, name: String) {
        let mut lobby = self.shared_lobby.get_mut();
        if let Some(player) = lobby.get_player_mut(Some(name.as_str()), None) {
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
        let mut lobby = self.shared_lobby.get_mut();
        let (steamid, chat_msg_id) = {
            if let Some(player) = lobby.get_player(Some(name.as_str()), None) {
                (player.steamid, lobby.chat_msg_id)
            } else {
                log::warn!("Player not found: '{}'", name);
                return;
            }
        };

        lobby.chat.push(LobbyChat {
            chat_msg_id,
            when,
            steamid,
            player_name: name,
            message: message.trim().to_string(),
            translated_message: None,
            dead,
            team,
        });

        lobby.chat_msg_id += 1;
    }

    // Go through the recently_left_players
    // and remove those who are still active
    // and remove those who are older than a certain seconds
    fn purge_old_players(&mut self) {
        let when = Local::now();
        let mut lobby = self.shared_lobby.get_mut();

        let lobby_steamids: HashSet<SteamID> = lobby.players.iter().map(|p| p.steamid).collect();

        let mut recently_left_to_keep: Vec<Player> = vec![];
        for player in lobby.recently_left_players.iter() {
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

        lobby.recently_left_players = recently_left_to_keep;
    }

    fn translate_chat(&mut self) {
        let mut lobby = self.shared_lobby.get_mut();
        for chat in lobby.chat.iter_mut() {
            if chat.translated_message.is_some() {
                continue;
            }

            let translated_message = self
                .text_translator
                .translate_sync(&chat.message, "en", "")
                .unwrap_or_else(|e| {
                    log::error!("Error translating chat message: {:?}", e);
                    chat.message.clone()
                });

            chat.translated_message = Some(translated_message);
        }
    }
}
