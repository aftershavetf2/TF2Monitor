use super::LobbyKill;
use super::chat_translation::ChatTranslation;
use super::shared_lobby::SharedLobby;
use super::{LobbyChat, Player, PlayerKill};
use crate::config::LOBBY_LOOP_DELAY;
use crate::tf2::lobby::AccountAge;
use crate::tf2::rcon::{G15DumpPlayerOutput, G15PlayerData};
use crate::tf2::steamapi::SteamApiMsg;
use crate::tf2bd::Tf2bdMsg;
use crate::{
    appbus::{AppBus, AppEventMsg},
    db::{connection::DbPool, queries},
    models::{
        app_settings::{AppSettings, TranslationOptions},
        player_profile::ProfileEvent,
        steamid::SteamID,
    },
    tf2::logfile::LogLine,
};
use bus::BusReader;
use chrono::prelude::*;
use diesel::Connection;
use std::collections::{HashMap, HashSet};
use std::{
    sync::{Arc, Mutex},
    thread::{self, sleep},
};

/// The number of seconds a player can be in the recently_left_players collection
const RECENTLY_LEFT_TIMEOUT_REMOVAL_SECONDS: i64 = 90;

pub struct LobbyThread {
    logfile_bus_rx: BusReader<LogLine>,
    steamapi_bus_rx: BusReader<SteamApiMsg>,
    tf2bd_bus_rx: BusReader<Tf2bdMsg>,
    g15_bus_rx: BusReader<G15DumpPlayerOutput>,
    app_event_bus_rx: BusReader<AppEventMsg>,
    shared_lobby: SharedLobby,
    bus: Arc<Mutex<AppBus>>,
    db: DbPool,
    saved_server_names: HashMap<SteamID, String>,
    awaiting_map_after_lobby_reset: bool,

    chat_translation: ChatTranslation,
    translation_options: Option<TranslationOptions>,
}

/// Start the background thread for the lobby module
pub fn start(
    settings: &AppSettings,
    bus: &Arc<Mutex<AppBus>>,
    db: &DbPool,
) -> thread::JoinHandle<()> {
    let mut lobby_thread = LobbyThread::new(settings, bus, db);

    thread::spawn(move || lobby_thread.run())
}

impl LobbyThread {
    pub fn new(settings: &AppSettings, bus: &Arc<Mutex<AppBus>>, db: &DbPool) -> Self {
        let logfile_bus_rx = bus.lock().unwrap().logfile_bus.add_rx();
        let steamapi_bus_rx = bus.lock().unwrap().steamapi_bus.add_rx();
        let tf2bd_bus_rx = bus.lock().unwrap().tf2bd_bus.add_rx();
        let g15_bus_rx = bus.lock().unwrap().g15_report_bus.add_rx();
        let app_event_bus_rx = bus.lock().unwrap().app_event_bus.add_rx();
        let shared_lobby = bus.lock().unwrap().shared_lobby.clone();

        Self {
            logfile_bus_rx,
            steamapi_bus_rx,
            tf2bd_bus_rx,
            g15_bus_rx,
            app_event_bus_rx,
            shared_lobby,
            bus: Arc::clone(bus),
            db: db.clone(),
            saved_server_names: HashMap::new(),
            awaiting_map_after_lobby_reset: false,

            chat_translation: ChatTranslation::default(),
            translation_options: settings.chat_translation_options(),
        }
    }

    pub fn run(&mut self) {
        log::info!("Lobby background thread started");

        loop {
            self.process_bus();

            let mut lobby = self.shared_lobby.get();
            lobby.update_friendships();

            self.chat_translation
                .update(&mut lobby, self.translation_options);
            self.shared_lobby.set(lobby);

            sleep(LOBBY_LOOP_DELAY);
        }
    }

    fn process_bus(&mut self) {
        self.process_app_events();
        self.purge_old_players();
        self.process_g15_bus();
        self.process_logfile_bus();
        self.process_steamapi_bus();
        self.process_tf2bd_bus();
    }

    fn process_app_events(&mut self) {
        while let Ok(AppEventMsg::UpdatedSettings(settings)) = self.app_event_bus_rx.try_recv() {
            let options = settings.chat_translation_options();
            if options.map(|options| options.model)
                != self.translation_options.map(|options| options.model)
            {
                self.chat_translation.retry_worker();
            }
            self.translation_options = options;
        }
    }

    fn process_g15_bus(&mut self) {
        while let Ok(g15_dump) = self.g15_bus_rx.try_recv() {
            self.process_g15_dump(g15_dump);
        }
    }

    fn process_g15_dump(&mut self, g15_dump: G15DumpPlayerOutput) {
        let now = Local::now();
        let mut lobby = self.shared_lobby.get();
        if g15_dump.generation != lobby.g15_generation {
            return;
        }

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

        self.shared_lobby.set(lobby);
        self.saved_server_names
            .retain(|id, _| g15_steamids.contains(id));
        if let Err(error) = self.persist_server_names(&g15_dump.players) {
            log::warn!("Failed to save lobby player names: {error}");
        }
    }

    fn persist_server_names(
        &mut self,
        players: &[G15PlayerData],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let names: Vec<_> = players
            .iter()
            .filter(|player| {
                player.steamid.is_valid()
                    && !player.name.trim().is_empty()
                    && self.saved_server_names.get(&player.steamid) != Some(&player.name)
            })
            .collect();
        if names.is_empty() {
            return Ok(());
        }

        let mut conn = self.db.get()?;
        let changed = conn.transaction::<_, diesel::result::Error, _>(|conn| {
            let mut changed = Vec::new();
            for player in &names {
                if queries::upsert_server_name(conn, player.steamid.to_u64() as i64, &player.name)?
                    > 0
                {
                    changed.push(player.steamid);
                }
            }
            Ok(changed)
        })?;
        for player in names {
            self.saved_server_names
                .insert(player.steamid, player.name.clone());
        }
        if !changed.is_empty() {
            let mut bus = self.bus.lock().unwrap();
            for id in changed {
                bus.profile_events.broadcast(ProfileEvent::Changed(id));
            }
        }
        Ok(())
    }

    fn merge_player_g15_data(lobby_player: &mut Player, player: &G15PlayerData) {
        lobby_player.steamid = player.steamid;

        lobby_player.id = player.id;
        lobby_player.name = player.name.clone();
        if let Some(team) = player.team {
            lobby_player.team = team;
        }
        lobby_player.alive = player.alive;
        lobby_player.ping_ms = player.ping_ms;
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
                    killer,
                    victim,
                    weapon,
                    crit,
                } => self.kill(killer, victim, weapon, crit),
                LogLine::Suicide { name } => self.suicide(name),
                LogLine::LobbyCreated => self.matchmaking_lobby_created(),
                LogLine::LobbyDestroyed => self.awaiting_map_after_lobby_reset = false,
                LogLine::MapLoaded => self.map_loaded(),
                LogLine::Chat {
                    name,
                    message,
                    dead,
                    team,
                } => self.chat(name, message, dead, team),
            }
        }
    }

    fn matchmaking_lobby_created(&mut self) {
        if self.awaiting_map_after_lobby_reset {
            return;
        }
        self.new_lobby();
        self.awaiting_map_after_lobby_reset = true;
    }

    fn map_loaded(&mut self) {
        // Matchmaking can announce the same session before the map loads.
        // Reset again if players scored on the previous map while waiting.
        if std::mem::take(&mut self.awaiting_map_after_lobby_reset) {
            let mut lobby = self.shared_lobby.get();
            let has_scores = lobby
                .players
                .iter()
                .chain(&lobby.recently_left_players)
                .any(|player| player.kills != 0 || player.deaths != 0);
            if !has_scores {
                // Even when the score reset is already done, invalidate any G15 data
                // collected on the previous map while the new one was loading.
                lobby.g15_generation += 1;
                lobby.players.clear();
                self.shared_lobby.set(lobby);
                return;
            }
        }
        self.new_lobby();
    }

    fn new_lobby(&mut self) {
        log::info!("*** Creating new lobby ***");

        let mut lobby = self.shared_lobby.get();
        let now = Local::now();

        lobby.creation_date = now;

        for player in lobby.players.iter_mut() {
            player.last_seen = now;
            player.clear_stats();
        }

        for player in lobby.recently_left_players.iter_mut() {
            player.clear_stats();
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

        lobby.session_id += 1;
        lobby.g15_generation += 1;
        lobby.lobby_id = format!("{}:{}", now.to_rfc3339(), lobby.session_id);

        lobby.players.clear();
        lobby.chat.clear();
        self.chat_translation.reset_lobby();
        lobby.kill_feed.clear();

        self.shared_lobby.set(lobby);
    }

    fn kill(&mut self, killer_name: String, victim_name: String, weapon: String, crit: bool) {
        let mut lobby = self.shared_lobby.get();

        let mut killer_steamid = None;
        let mut victim_steamid = None;

        // Change the counts of kills to the killer player
        if let Some(killer) = lobby.get_player_mut(Some(killer_name.as_str()), None) {
            killer_steamid = Some(killer.steamid);
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

        // Change the counts of deaths to the victim player
        if let Some(victim) = lobby.get_player_mut(Some(victim_name.as_str()), None) {
            victim_steamid = Some(victim.steamid);
            victim.deaths += 1;
            if crit {
                victim.crit_deaths += 1;
            }
        } else {
            log::warn!("Victim not found: '{}'", victim_name);
        }

        // Add the kill to the feed
        if let (Some(killer_steamid), Some(victim_steamid)) = (killer_steamid, victim_steamid) {
            lobby.kill_feed.push(LobbyKill {
                killer: killer_steamid,
                victim: victim_steamid,
                killer_name,
                victim_name,
                weapon,
                crit,
            });
        } else {
            log::warn!(
                "Killer or victim not found: '{}', '{}'",
                killer_name,
                victim_name
            );
        }

        self.shared_lobby.set(lobby);
    }

    fn suicide(&mut self, name: String) {
        let mut lobby = self.shared_lobby.get();

        if let Some(player) = lobby.get_player_mut(Some(name.as_str()), None) {
            player.deaths += 1;
        } else {
            log::warn!("Player not found: '{}'", name);
        }

        self.shared_lobby.set(lobby);
    }

    fn chat(&mut self, name: String, message: String, dead: bool, team: bool) {
        let mut lobby = self.shared_lobby.get();

        let steamid = {
            if let Some(player) = lobby.get_player(Some(name.as_str()), None) {
                player.steamid
            } else {
                log::warn!("Player not found: '{}'", name);
                return;
            }
        };

        lobby.chat.push(LobbyChat {
            steamid,
            player_name: name,
            message: message.trim().to_string(),
            translated_message: None,
            dead,
            team,
        });

        self.shared_lobby.set(lobby);
    }

    // Go through the recently_left_players
    // and remove those who are still active
    // and remove those who are older than a certain seconds
    fn purge_old_players(&mut self) {
        let when = Local::now();
        let mut lobby = self.shared_lobby.get();

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

        self.shared_lobby.set(lobby);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lobby_log_signals_reset_scores_on_a_new_match_and_same_map_reload() {
        use crate::{models::player_profile::test_app, tf2::logfile::line_parser::LogLineParser};

        let app = test_app();
        let mut worker = LobbyThread::new(&app.app_settings, &app.bus, &app.db);
        let parser = LogLineParser::default();
        let scored_player = Player {
            name: "Player".into(),
            kills: 7,
            deaths: 4,
            crit_kills: 2,
            crit_deaths: 1,
            kills_with: vec![PlayerKill {
                weapon: "rocketlauncher".into(),
                crit: true,
            }],
            ..Player::default()
        };
        let mut lobby = worker.shared_lobby.get();
        lobby.players.push(scored_player.clone());
        let initial_session = lobby.session_id;
        worker.shared_lobby.set(lobby);

        for line in [
            "Lobby created",
            concat!(
                "Differing lobby received. Lobby: [A:1:123:456]/Match2/Lobby3 ",
                "CurrentlyAssigned: [A:1:123:456]/Match1/Lobby3 ",
                "ConnectedToMatchServer: 1 HasLobby: 1 AssignedMatchEnded: 0",
            ),
            "Map: pl_badwater",
        ] {
            app.bus
                .lock()
                .unwrap()
                .send_logline(parser.parse_line(line).unwrap());
        }
        worker.process_logfile_bus();

        let mut lobby = worker.shared_lobby.get();
        assert_eq!(lobby.session_id, initial_session + 1);
        let player = &lobby.recently_left_players[0];
        assert_eq!(
            (
                player.kills,
                player.deaths,
                player.crit_kills,
                player.crit_deaths
            ),
            (0, 0, 0, 0)
        );
        assert!(player.kills_with.is_empty());

        lobby.players.push(scored_player);
        worker.shared_lobby.set(lobby);
        app.bus
            .lock()
            .unwrap()
            .send_logline(parser.parse_line("Map: pl_badwater").unwrap());
        worker.process_logfile_bus();
        let lobby = worker.shared_lobby.get();
        assert_eq!(lobby.session_id, initial_session + 2);
        assert!(lobby.players.is_empty());
        assert!(
            lobby
                .recently_left_players
                .iter()
                .all(|player| player.kills == 0 && player.deaths == 0)
        );
    }

    #[test]
    fn map_load_retires_the_g15_roster_and_rejects_queued_reports() {
        use crate::models::{player_profile::test_app, steamid::MIN_STEAMID64};

        let app = test_app();
        let mut worker = LobbyThread::new(&app.app_settings, &app.bus, &app.db);
        let report = |generation, account_id, name: &str| G15DumpPlayerOutput {
            generation,
            players: vec![G15PlayerData {
                steamid: SteamID::from_u64(MIN_STEAMID64 + account_id),
                name: name.into(),
                ..Default::default()
            }],
        };

        let previous_generation = app.shared_lobby.g15_generation();
        worker.matchmaking_lobby_created();
        let announced = app.shared_lobby.get();
        assert_eq!(announced.g15_generation, previous_generation + 1);

        worker.process_g15_dump(report(previous_generation, 1, "Late lobby report"));
        assert!(app.shared_lobby.get().players.is_empty());
        worker.process_g15_dump(report(announced.g15_generation, 1, "Previous map"));
        assert_eq!(app.shared_lobby.get().players.len(), 1);

        app.bus.lock().unwrap().g15_report_bus.broadcast(report(
            announced.g15_generation,
            1,
            "Queued report",
        ));
        worker.map_loaded();
        let loaded = app.shared_lobby.get();
        assert_eq!(loaded.session_id, announced.session_id);
        assert_eq!(loaded.g15_generation, announced.g15_generation + 1);
        assert!(loaded.players.is_empty());

        worker.process_g15_bus();
        assert!(app.shared_lobby.get().players.is_empty());
        worker.process_g15_dump(report(loaded.g15_generation, 2, "Current map"));
        let lobby = app.shared_lobby.get();
        assert_eq!(lobby.players.len(), 1);
        assert_eq!(lobby.players[0].name, "Current map");
    }

    #[test]
    fn new_lobbies_have_distinct_sessions_even_on_the_same_day() {
        let settings = AppSettings::default();
        let bus = Arc::new(Mutex::new(AppBus::new(settings.self_steamid64)));
        let app = crate::models::player_profile::test_app();
        let mut worker = LobbyThread::new(&settings, &bus, &app.db);
        let initial = worker.shared_lobby.get();
        worker.new_lobby();
        let first = worker.shared_lobby.get();
        worker.new_lobby();
        let second = worker.shared_lobby.get();
        assert!(initial.session_id < first.session_id);
        assert!(first.session_id < second.session_id);
        assert_ne!(initial.lobby_id, first.lobby_id);
        assert_ne!(first.lobby_id, second.lobby_id);
    }

    #[test]
    fn server_name_updates_the_cached_profile_and_survives_steam_refresh() {
        use crate::{
            db::entities::NewAccount,
            models::{player_profile::test_app, steamid::MIN_STEAMID64},
        };

        let mut app = test_app();
        let mut worker = LobbyThread::new(&app.app_settings, &app.bus, &app.db);
        let steamid = SteamID::from_u64(MIN_STEAMID64 + 7);
        let dump = |name: &str| G15DumpPlayerOutput {
            generation: 0,
            players: vec![G15PlayerData {
                steamid,
                name: name.into(),
                ..Default::default()
            }],
        };
        let refresh_steam_profile = || {
            queries::upsert_account(
                &mut app.db.get().unwrap(),
                NewAccount {
                    steam_id: steamid.to_u64() as i64,
                    name: "Steam profile name".into(),
                    avatar_thumb_url: "thumb".into(),
                    avatar_full_url: "full".into(),
                    public_profile: true,
                    last_updated: 1,
                    created_date: None,
                    friends_fetched: None,
                    comments_fetched: None,
                    playtimes_fetched: None,
                    steam_bans_last_fetched: None,
                },
            )
            .unwrap();
        };
        refresh_steam_profile();
        assert_eq!(
            app.player_profile(steamid).unwrap().name,
            "Steam profile name"
        );

        worker.process_g15_dump(dump("First server name"));
        worker.process_g15_dump(dump("Renamed in server"));
        worker.process_g15_dump(dump(""));
        worker.process_g15_dump(G15DumpPlayerOutput::default());
        refresh_steam_profile();
        app.get_latest_lobby();

        let profile = app.player_profile(steamid).unwrap();
        assert_eq!(profile.name, "Renamed in server");
        assert_eq!(profile.steam_info.unwrap().avatar_thumb, "thumb");

        let new_id = SteamID::from_u64(MIN_STEAMID64 + 8);
        worker.process_g15_dump(G15DumpPlayerOutput {
            generation: 0,
            players: vec![G15PlayerData {
                steamid: new_id,
                name: "New player".into(),
                ..Default::default()
            }],
        });
        let profile = app.player_profile(new_id).unwrap();
        assert_eq!(profile.name, "New player");
        assert!(profile.steam_info.is_none());
    }

    #[test]
    fn chat_translation_follows_saved_settings_and_live_updates() {
        use crate::models::app_settings::{TranslationMode, TranslationModel};

        let mut settings = AppSettings::default();
        assert!(!settings.translate_chat);
        let mut saved = serde_json::to_value(&settings).unwrap();
        let fields = saved.as_object_mut().unwrap();
        fields.remove("translate_chat");
        fields.remove("chat_translation_mode");
        fields.remove("chat_translation_model");
        let older_settings: AppSettings = serde_json::from_value(saved).unwrap();
        assert!(!older_settings.translate_chat);
        assert_eq!(older_settings.chat_translation_mode, TranslationMode::Fast);
        assert_eq!(
            older_settings.chat_translation_model,
            TranslationModel::Standard
        );

        settings.translate_chat = true;
        settings.chat_translation_mode = TranslationMode::Beam;
        settings.chat_translation_model = TranslationModel::Large;
        let saved = serde_json::to_string(&settings).unwrap();
        let mut settings: AppSettings = serde_json::from_str(&saved).unwrap();
        let bus = Arc::new(Mutex::new(AppBus::new(settings.self_steamid64)));
        let app = crate::models::player_profile::test_app();
        let mut lobby_thread = LobbyThread::new(&settings, &bus, &app.db);
        assert_eq!(
            lobby_thread.translation_options,
            Some(TranslationOptions {
                mode: TranslationMode::Beam,
                model: TranslationModel::Large,
            })
        );

        for (enabled, model, mode) in [
            (true, TranslationModel::Standard, TranslationMode::Beam),
            (true, TranslationModel::Standard, TranslationMode::Fast),
            (false, TranslationModel::Large, TranslationMode::Fast),
            (true, TranslationModel::Large, TranslationMode::Fast),
        ] {
            settings.translate_chat = enabled;
            settings.chat_translation_model = model;
            settings.chat_translation_mode = mode;
            bus.lock()
                .unwrap()
                .app_event_bus
                .broadcast(AppEventMsg::UpdatedSettings(settings.clone()));
            lobby_thread.process_app_events();
            assert_eq!(
                lobby_thread.translation_options,
                enabled.then_some(TranslationOptions { mode, model })
            );
        }
    }
}
