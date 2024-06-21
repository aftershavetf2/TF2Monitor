use super::{SteamApi, SteamPlayerBan};
use crate::{
    appbus::AppBus,
    models::{app_settings::AppSettings, steamid::SteamID},
    tf2::{
        lobby::{AccountAge, Lobby, Player, PlayerSteamInfo},
        steamapi::SteamApiMsg,
    },
};
use bus::BusReader;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    thread::{self, sleep},
};

/// The delay between loops in run()
const LOOP_DELAY: std::time::Duration = std::time::Duration::from_millis(100);

/// For each loop, fetch this many players' TF2 playtimes
const NUM_PLAYTIMES_TO_FETCH: usize = 1;

/// For each loop, fetch this many players' friends list
const NUM_FRIENDS_TO_FETCH: usize = 1;

/// For each loop, approximate  this many players' account ages
const NUM_ACCOUNT_AGES_TO_APPROX: usize = 1;

/// Start the background thread for the rcon module
pub fn start(settings: &AppSettings, bus: &Arc<Mutex<AppBus>>) -> thread::JoinHandle<()> {
    let mut steamapi_thread = SteamApiThread::new(settings, bus);

    thread::spawn(move || steamapi_thread.run())
}

struct SteamApiCache {
    summaries: HashMap<SteamID, PlayerSteamInfo>,
    friends: HashMap<SteamID, HashSet<SteamID>>,
    playtimes: HashMap<SteamID, u32>,
    steam_bans: HashMap<SteamID, SteamPlayerBan>,
}

impl SteamApiCache {
    fn new() -> Self {
        Self {
            summaries: HashMap::new(),
            friends: HashMap::new(),
            playtimes: HashMap::new(),
            steam_bans: HashMap::new(),
        }
    }

    fn get_friends(&self, steamid: SteamID) -> Option<&HashSet<SteamID>> {
        self.friends.get(&steamid)
    }

    fn set_friends(&mut self, steamid: SteamID, friends: HashSet<SteamID>) {
        self.friends.insert(steamid, friends);
    }

    fn get_summary(&self, steamid: SteamID) -> Option<&PlayerSteamInfo> {
        self.summaries.get(&steamid)
    }

    fn set_summary(&mut self, summary: PlayerSteamInfo) {
        self.summaries.insert(summary.steamid, summary);
    }

    fn get_playtime(&self, steamid: SteamID) -> Option<&u32> {
        self.playtimes.get(&steamid)
    }

    fn set_playtime(&mut self, steamid: SteamID, playtime: u32) {
        self.playtimes.insert(steamid, playtime);
    }

    fn get_steam_ban(&self, steamid: SteamID) -> Option<&SteamPlayerBan> {
        self.steam_bans.get(&steamid)
    }

    fn set_steam_ban(&mut self, steamid: SteamID, steam_ban: SteamPlayerBan) {
        self.steam_bans.insert(steamid, steam_ban);
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

        if let Ok(lobby) = self.lobby_bus_rx.try_recv() {
            // log::info!("process_bus - received lobby");
            self.fetch_summaries(&lobby);
            self.fetch_steam_bans(&lobby);
            self.fetch_friends(&lobby);
            self.fetch_playtimes(&lobby);
            self.approximate_account_ages(&lobby);
        }

        while let Ok(_lobby) = self.lobby_bus_rx.try_recv() {}
    }

    fn fetch_summaries(&mut self, lobby: &Lobby) {
        let mut summaries_to_fetch = Vec::new();

        for player in lobby.players.iter() {
            if player.steam_info.is_none() {
                // First check cache
                if let Some(summary) = self.steam_api_cache.get_summary(player.steamid) {
                    // log::info!("Fetched from cache summary of {}", player.name);
                    self.send(SteamApiMsg::PlayerSummary(summary.clone()));
                } else {
                    // Bulk fetch from Steam API below
                    summaries_to_fetch.push(player.steamid);
                }
            }
        }

        if let Some(infos) = self.steam_api.get_player_summaries(summaries_to_fetch) {
            for info in infos {
                if let Some(steamid) = SteamID::from_u64_string(&info.steamid) {
                    let public_profile = matches!(info.communityvisibilitystate, 3);

                    let info = PlayerSteamInfo {
                        steamid,
                        public_profile,
                        name: info.personaname.clone(),
                        avatar: info.avatar.clone(),
                        avatarmedium: info.avatarmedium.clone(),
                        avatarfull: info.avatarfull.clone(),
                        account_age: info.get_account_age(),
                    };

                    // Add to cache and then send
                    self.steam_api_cache.set_summary(info.clone());
                    self.send(SteamApiMsg::PlayerSummary(info));
                }
            }
        }
    }

    fn fetch_friends(&mut self, lobby: &Lobby) {
        let players = self.get_players_without_friends(lobby);
        for player in players {
            let steamid = player.steamid;

            if player.steam_info.is_some() {
                // First check cache
                if let Some(friends) = self.steam_api_cache.get_friends(steamid) {
                    // log::info!("Fetched from cache friends of {}", player.name);
                    self.send(SteamApiMsg::FriendsList(steamid, friends.clone()));
                    continue;
                }

                // Not in cache, fetch from Steam API and put in cache
                log::info!("Fetching friends of {}", player.name);
                if let Some(friends) = self.steam_api.get_friendlist(steamid) {
                    self.steam_api_cache.set_friends(steamid, friends.clone());
                    self.send(SteamApiMsg::FriendsList(steamid, friends.clone()));
                }
            }
        }
    }

    fn fetch_playtimes(&mut self, lobby: &Lobby) {
        let players = self.get_players_without_playtime(lobby);
        for player in players {
            let steamid = player.steamid;

            // First check cache
            if let Some(playtime) = self.steam_api_cache.get_playtime(steamid) {
                // log::info!("Fetched from cache playtime for {}", player.name);
                self.send(SteamApiMsg::Tf2Playtime(steamid, *playtime));
                continue;
            }

            // Not in cache, fetch from Steam API and put in cache
            log::info!("Fetching playtime for {}", player.name);
            if let Some(playtime) = self.steam_api.get_tf2_play_minutes(steamid) {
                self.steam_api_cache.set_playtime(steamid, playtime);
                self.send(SteamApiMsg::Tf2Playtime(steamid, playtime));
            }
        }
    }

    fn fetch_steam_bans(&mut self, lobby: &Lobby) {
        let mut bans_to_fetch = Vec::new();

        for player in lobby.players.iter() {
            if player.steam_bans.is_none() {
                // First check cache
                if let Some(ban) = self.steam_api_cache.get_steam_ban(player.steamid) {
                    // log::info!("Fetched from cache ban of {}", player.name);
                    self.send(SteamApiMsg::SteamBans(player.steamid, ban.clone()));
                } else {
                    // Bulk fetch from Steam API below
                    bans_to_fetch.push(player.steamid);
                }
            }
        }

        if let Some(bans) = self.steam_api.get_bans(bans_to_fetch) {
            for ban in bans {
                // Add to cache and then send
                self.steam_api_cache.set_steam_ban(ban.steamid, ban.clone());
                self.send(SteamApiMsg::SteamBans(ban.steamid, ban));
            }
        }
    }

    fn get_players_without_friends<'a>(&self, lobby: &'a Lobby) -> Vec<&'a Player> {
        lobby
            .players
            .iter()
            .filter(|p| p.friends.is_none())
            .take(NUM_FRIENDS_TO_FETCH)
            .collect()
    }

    fn get_players_without_playtime<'a>(&self, lobby: &'a Lobby) -> Vec<&'a Player> {
        lobby
            .players
            .iter()
            .filter(|p| p.tf2_play_minutes.is_none())
            .take(NUM_PLAYTIMES_TO_FETCH)
            .collect()
    }

    fn get_players_without_account_age<'a>(&self, lobby: &'a Lobby) -> Vec<&'a Player> {
        lobby
            .players
            .iter()
            .filter(|p: &&Player| p.steam_info.is_some())
            .filter(|p: &&Player| p.steam_bans.is_some())
            .filter(|p: &&Player| p.account_age == AccountAge::Private)
            .take(NUM_ACCOUNT_AGES_TO_APPROX)
            .collect()
    }

    fn approximate_account_ages(&mut self, lobby: &Lobby) {
        let players = self.get_players_without_account_age(lobby);
        for player in players {
            self.approximate_account_age(player);
        }
    }

    fn approximate_account_age(&mut self, player: &Player) {
        const NEIGHBORHOOD_SIZE: u64 = 49;

        log::info!(
            "Approximating account age for {} {}",
            player.name,
            player.steamid.to_u64()
        );
        let steamid = player.steamid.to_u64();

        let mut ids: Vec<SteamID> = Vec::new();
        for id in 1..NEIGHBORHOOD_SIZE {
            let id = steamid - id;
            ids.push(SteamID::from_u64(id));

            let id = steamid + id;
            ids.push(SteamID::from_u64(id));
        }

        let accounts = self.steam_api.get_player_summaries(ids);
        if let Some(accounts) = accounts {
            for account in accounts {
                if let Some(account_age) = account.get_account_age() {
                    // Found a neighbor with public profile
                    log::info!(
                        "Found neighbor with public profile for {}: {}",
                        player.name,
                        account.steamid
                    );
                    self.send(SteamApiMsg::ApproxAccountAge(
                        player.steamid,
                        AccountAge::Approx(account_age),
                    ));

                    return;
                }
            }
        }

        log::info!("No neighbors with public profile found for {}", player.name);

        self.send(SteamApiMsg::ApproxAccountAge(
            player.steamid,
            AccountAge::Unknown,
        ));
    }
}
