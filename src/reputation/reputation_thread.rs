use super::{get_reputation, Reputation};
use crate::{
    appbus::AppBus,
    models::{app_settings::AppSettings, steamid::SteamID},
    tf2::{
        lobby::{Lobby, Player},
        steamapi::SteamApiMsg,
    },
};
use bus::BusReader;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread::{self, sleep},
};

/// The delay between loops in run()
const LOOP_DELAY: std::time::Duration = std::time::Duration::from_millis(100);

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
    lobby_bus_rx: BusReader<Lobby>,
    reputation_cache: ReputationCache,
}

impl ReputationThread {
    pub fn new(_settings: &AppSettings, bus: &Arc<Mutex<AppBus>>) -> Self {
        let lobby_bus_rx = bus.lock().unwrap().lobby_report_bus.add_rx();

        Self {
            bus: Arc::clone(bus),
            lobby_bus_rx,
            reputation_cache: ReputationCache::new(),
        }
    }

    pub fn run(&mut self) {
        log::info!("SteamAPi background thread started");

        loop {
            self.process_bus();

            sleep(LOOP_DELAY);
        }
    }

    fn send(&mut self, msg: SteamApiMsg) {
        self.bus.lock().unwrap().steamapi_bus.broadcast(msg);
    }

    fn process_bus(&mut self) {
        if let Ok(lobby) = self.lobby_bus_rx.try_recv() {
            // log::info!("process_bus - received lobby");
            self.calculate_reputations(&lobby);
        }

        while let Ok(_lobby) = self.lobby_bus_rx.try_recv() {}
    }

    fn find_player_to_process<'a>(&mut self, lobby: &'a Lobby) -> Option<&'a Player> {
        lobby
            .players
            .iter()
            .find(|&player| player.reputation.is_none())
    }

    fn calculate_reputations(&mut self, lobby: &Lobby) {
        if let Some(player) = self.find_player_to_process(lobby) {
            if let Some(reputation) = self.reputation_cache.get(player.steamid) {
                self.send(SteamApiMsg::Reputation(reputation.clone()));
                return;
            }

            let reputation = get_reputation(player.steamid);

            self.reputation_cache.set(reputation.clone());

            self.send(SteamApiMsg::Reputation(reputation));
        }
    }
}
