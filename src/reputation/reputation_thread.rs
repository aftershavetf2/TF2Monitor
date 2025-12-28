use super::{get_reputation, Reputation};
use crate::config::{NUM_REPUTATIONS_TO_FETCH, REPUTATION_LOOP_DELAY};
use crate::{
    appbus::AppBus,
    models::{app_settings::AppSettings, steamid::SteamID},
    tf2::{lobby::Lobby, steamapi::SteamApiMsg},
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread::{self, sleep},
};

pub fn start(settings: &AppSettings, bus: &Arc<Mutex<AppBus>>) -> thread::JoinHandle<()> {
    let mut reputation_thread = ReputationThread::new(settings, bus);

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
}

impl ReputationThread {
    pub fn new(_settings: &AppSettings, bus: &Arc<Mutex<AppBus>>) -> Self {
        let shared_lobby = bus.lock().unwrap().shared_lobby.clone();

        Self {
            bus: Arc::clone(bus),
            shared_lobby,
            reputation_cache: ReputationCache::new(),
        }
    }

    pub fn run(&mut self) {
        log::info!("SteamAPi background thread started");

        loop {
            self.get_latest_lobby();

            sleep(REPUTATION_LOOP_DELAY);
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
                return;
            }

            if !one_fetched {
                one_fetched = true;

                let reputation = get_reputation(player.steamid);

                self.reputation_cache.set(reputation.clone());

                self.send(SteamApiMsg::Reputation(reputation));
            }
        }
    }
}
