use super::{get_reputation, sourcebans, Reputation};
use crate::config::{NUM_REPUTATIONS_TO_FETCH, REPUTATION_LOOP_DELAY};
use crate::db::db::DbPool;
use crate::db::entities::{NewBan, NewBanSource};
use crate::db::queries;
use crate::{
    appbus::AppBus,
    models::{app_settings::AppSettings, steamid::SteamID},
    tf2::{lobby::Lobby, steamapi::SteamApiMsg},
};
use chrono::Utc;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread::{self, sleep},
};

pub fn start(settings: &AppSettings, bus: &Arc<Mutex<AppBus>>, db: &DbPool) -> thread::JoinHandle<()> {
    let mut reputation_thread = ReputationThread::new(settings, bus, db);

    thread::spawn(move || reputation_thread.run())
}

struct ReputationCache {
    cache: HashMap<SteamID, Reputation>,
}

impl ReputationCache {
    fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    fn get(&self, steamid: SteamID) -> Option<&Reputation> {
        self.cache.get(&steamid)
    }

    fn set(&mut self, reputation: Reputation) {
        self.cache.insert(reputation.steamid, reputation);
    }
}

pub struct ReputationThread {
    bus: Arc<Mutex<AppBus>>,
    shared_lobby: crate::tf2::lobby::shared_lobby::SharedLobby,
    reputation_cache: ReputationCache,
    db: DbPool,
}

impl ReputationThread {
    pub fn new(_settings: &AppSettings, bus: &Arc<Mutex<AppBus>>, db: &DbPool) -> Self {
        let shared_lobby = bus.lock().unwrap().shared_lobby.clone();

        Self {
            bus: Arc::clone(bus),
            shared_lobby,
            reputation_cache: ReputationCache::new(),
            db: db.clone(),
        }
    }

    pub fn run(&mut self) {
        log::info!("SteamAPi background thread started");

        // Persist all configured ban sources to the database on startup
        self.persist_ban_sources();

        loop {
            self.get_latest_lobby();

            sleep(REPUTATION_LOOP_DELAY);
        }
    }

    fn persist_ban_sources(&self) {
        let sources = sourcebans::get_sources();

        if let Ok(mut conn) = self.db.get() {
            for source in sources {
                let parser_str = match source.parser {
                    sourcebans::SourceBanParser::Ul => "Ul",
                    sourcebans::SourceBanParser::Table => "Table",
                };

                let new_source = NewBanSource {
                    name: source.name.clone(),
                    url: source.url.clone(),
                    parser: parser_str.to_string(),
                    last_checked: None,
                    active: true,
                };

                if let Err(e) = queries::upsert_ban_source(&mut conn, new_source) {
                    log::error!("Failed to persist ban source {}: {}", source.name, e);
                }
            }
        }
    }

    fn send(&mut self, msg: SteamApiMsg) {
        self.bus.lock().unwrap().steamapi_bus.broadcast(msg);
    }

    fn get_latest_lobby(&mut self) {
        // Get a copy of the current lobby state
        let lobby = self.shared_lobby.get();
        self.calculate_reputations(&lobby);
    }

    fn calculate_reputations(&mut self, lobby: &Lobby) {
        let mut one_fetched = false;

        let players = lobby
            .players
            .iter()
            .filter(|player| player.reputation.is_none())
            .take(NUM_REPUTATIONS_TO_FETCH);
        for player in players {
            if player.reputation.is_some() {
                continue;
            }

            if let Some(reputation) = self.reputation_cache.get(player.steamid) {
                self.send(SteamApiMsg::Reputation(reputation.clone()));
                continue;
            }

            if !one_fetched {
                // Check if reputation was fetched recently (within 7 days)
                if self.should_fetch_reputation(player.steamid) {
                    one_fetched = true;

                    let reputation = get_reputation(player.steamid);

                    self.reputation_cache.set(reputation.clone());

                    self.send(SteamApiMsg::Reputation(reputation.clone()));

                    // Persist bans to database and update reputation_fetched timestamp
                    if let Ok(mut conn) = self.db.get() {
                        let current_time = Utc::now().timestamp();

                        for ban in &reputation.bans {
                            // Parse ban_length to determine if permanent
                            let permanent = ban.ban_length.to_lowercase().contains("permanent")
                                || ban.ban_length.to_lowercase().contains("never");

                            let new_ban = NewBan {
                                steam_id: ban.steamid.to_u64() as i64,
                                source: ban.source.clone(),
                                ban_type: String::from("sourcebans"),
                                reason: Some(ban.reason.clone()),
                                created_date: current_time,
                                expires_date: None, // Could parse ban_length here if needed
                                permanent,
                            };

                            if let Err(e) = queries::insert_ban(&mut conn, new_ban) {
                                log::debug!("Ban already exists or error: {}", e);
                            }
                        }

                        // Update reputation_fetched timestamp
                        if let Err(e) = queries::update_account_reputation_fetched(
                            &mut conn,
                            player.steamid.to_u64() as i64,
                            current_time,
                        ) {
                            log::error!(
                                "Failed to update reputation_fetched for {}: {}",
                                player.steamid.to_u64(),
                                e
                            );
                        }
                    }
                }
            }
        }
    }

    /// Check if reputation should be fetched for this steam_id.
    /// Returns true if reputation hasn't been fetched, or was fetched more than 7 days ago.
    fn should_fetch_reputation(&self, steamid: SteamID) -> bool {
        if let Ok(mut conn) = self.db.get() {
            if let Ok(Some(account)) =
                queries::get_account_by_steam_id(&mut conn, steamid.to_u64() as i64)
            {
                if let Some(reputation_fetched) = account.reputation_fetched {
                    let current_time = Utc::now().timestamp();
                    let seven_days_in_seconds = 7 * 24 * 60 * 60;
                    let time_since_fetch = current_time - reputation_fetched;

                    if time_since_fetch < seven_days_in_seconds {
                        log::debug!(
                            "Skipping reputation fetch for {} - last fetched {} days ago",
                            steamid.to_u64(),
                            time_since_fetch / (24 * 60 * 60)
                        );
                        return false;
                    }
                }
            }
        }

        // If account doesn't exist or reputation_fetched is None, or DB error, fetch it
        true
    }
}
