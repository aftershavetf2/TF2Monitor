use super::SteamApi;
use crate::{
    appbus::AppBus,
    models::{app_settings::AppSettings, steamid::SteamID},
    tf2::{
        lobby::{Lobby, Player, PlayerSteamInfo},
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
const LOOP_DELAY: std::time::Duration = std::time::Duration::from_millis(500);

/// Start the background thread for the rcon module
pub fn start(settings: &AppSettings, bus: &Arc<Mutex<AppBus>>) -> thread::JoinHandle<()> {
    let mut steamapi_thread = SteamApiThread::new(settings, bus);

    thread::spawn(move || steamapi_thread.run())
}

struct SteamApiCache {
    summaries: HashMap<SteamID, PlayerSteamInfo>,
    friends: HashMap<SteamID, Vec<SteamID>>,
}

impl SteamApiCache {
    fn new() -> Self {
        Self {
            summaries: HashMap::new(),
            friends: HashMap::new(),
        }
    }

    fn get_friends(&self, steamid: SteamID) -> Option<&Vec<SteamID>> {
        self.friends.get(&steamid)
    }

    fn add_friends(&mut self, steamid: SteamID, friends: Vec<SteamID>) {
        self.friends.insert(steamid, friends);
    }

    fn get_summary(&self, steamid: SteamID) -> Option<&PlayerSteamInfo> {
        self.summaries.get(&steamid)
    }

    fn add_summary(&mut self, summary: PlayerSteamInfo) {
        self.summaries.insert(summary.steamid, summary);
    }
}

pub struct SteamApiThread {
    bus: Arc<Mutex<AppBus>>,
    lobby_bus_rx: BusReader<Lobby>,
    steam_api: SteamApi,
    steam_api_cache: SteamApiCache,
}

impl SteamApiThread {
    pub fn new(settings: &AppSettings, bus: &Arc<Mutex<AppBus>>) -> Self {
        let lobby_bus_rx = bus.lock().unwrap().lobby_report_bus.add_rx();

        Self {
            bus: Arc::clone(bus),
            lobby_bus_rx,
            steam_api: SteamApi::new(settings),
            steam_api_cache: SteamApiCache::new(),
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
        // To fetch additional info from Steam Web Api a key is needed
        if !self.steam_api.has_key() {
            return;
        }

        while let Ok(lobby) = self.lobby_bus_rx.try_recv() {
            // log::info!("process_bus - received lobby");
            self.fetch_summaries(&lobby);
            self.fetch_friends(&lobby);
        }
    }

    fn fetch_summaries(&mut self, lobby: &Lobby) {
        let players = self.get_players_without_steam_infos(lobby);
        if players.is_empty() {
            // log::info!("fetch_summaries - No players to process");
            return;
        }

        let mut summaries_to_fetch = Vec::new();

        for player in players {
            if player.steam_info.is_some() {
                // First check cache
                if let Some(summary) = self.steam_api_cache.get_summary(player.steamid) {
                    log::info!("Fetched from cache summary of {}", player.name);
                    self.send(SteamApiMsg::PlayerSummary(summary.clone()));
                    continue;
                }
            }

            // Bulk fetch from Steam API below
            summaries_to_fetch.push(player.steamid);
        }

        if summaries_to_fetch.is_empty() {
            return;
        }

        if let Some(infos) = self.steam_api.get_player_summaries(summaries_to_fetch) {
            for info in infos {
                if let Some(steamid) = SteamID::from_u64_string(&info.steamid) {
                    let info = PlayerSteamInfo {
                        steamid,
                        name: info.personaname.clone(),
                        avatar: info.avatar.clone(),
                        avatarmedium: info.avatarmedium.clone(),
                        avatarfull: info.avatarfull.clone(),
                        account_age: info.get_account_age(),
                        friends: None,
                    };

                    // Add to cache and then send
                    self.steam_api_cache.add_summary(info.clone());
                    self.send(SteamApiMsg::PlayerSummary(info));
                }
            }
        }
    }

    fn fetch_friends(&mut self, lobby: &Lobby) {
        let players = self.get_players_without_friends(lobby, usize::MAX);
        if players.is_empty() {
            // log::info!("fetch_friends - No players to process");
            return;
        }

        for player in players {
            let steamid = player.steamid;

            if player.steam_info.is_some() {
                // First check cache
                if let Some(friends) = self.steam_api_cache.get_friends(steamid) {
                    log::info!("Fetched from cache friends of {}", player.name);
                    self.send(SteamApiMsg::FriendsList(steamid, friends.clone()));
                } else {
                    // Not in cache, fetch from Steam API and put in cache
                    log::info!("Fetching friends of {}", player.name);
                    if let Some(friends) = self.steam_api.get_friendlist(steamid) {
                        self.steam_api_cache.add_friends(steamid, friends.clone());
                        self.send(SteamApiMsg::FriendsList(steamid, friends.clone()));
                    }
                }
            }
        }
    }

    fn get_players_without_steam_infos<'a>(&self, lobby: &'a Lobby) -> Vec<&'a Player> {
        lobby
            .players
            .iter()
            .filter(|p| p.steam_info.is_none())
            .collect()
    }

    fn get_players_without_friends<'a>(&self, lobby: &'a Lobby, take_n: usize) -> Vec<&'a Player> {
        lobby
            .players
            .iter()
            .filter(|p| {
                if let Some(steam_info) = &p.steam_info {
                    steam_info.friends.is_none()
                } else {
                    false
                }
            })
            .take(take_n)
            .collect()
    }
}
