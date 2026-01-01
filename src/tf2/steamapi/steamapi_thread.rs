use super::{
    get_steam_comments::get_steam_profile_comments, SteamApi, SteamPlayerBan, SteamProfileComment,
};
use crate::config::{
    NUM_ACCOUNT_AGES_TO_APPROX, NUM_FRIENDS_TO_FETCH, NUM_PLAYTIMES_TO_FETCH,
    NUM_PROFILE_COMMENTS_TO_FETCH, STEAMAPI_LOOP_DELAY, STEAMAPI_RETRY_DELAY,
};
use crate::{
    appbus::AppBus,
    models::{app_settings::AppSettings, steamid::SteamID},
    tf2::{
        lobby::{AccountAge, Lobby, Player, PlayerSteamInfo, Tf2PlayMinutes},
        steamapi::SteamApiMsg,
    },
};
use sea_orm::DatabaseConnection;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    thread::{self, sleep},
};

/// Start the background thread for the rcon module
pub fn start(settings: &AppSettings, bus: &Arc<Mutex<AppBus>>, db: &DatabaseConnection) -> thread::JoinHandle<()> {
    let mut steamapi_thread = SteamApiThread::new(settings, bus, db);

    thread::spawn(move || steamapi_thread.run())
}

struct SteamApiCache {
    summaries: HashMap<SteamID, PlayerSteamInfo>,
    friends: HashMap<SteamID, HashSet<SteamID>>,
    playtimes: HashMap<SteamID, Tf2PlayMinutes>,
    steam_bans: HashMap<SteamID, SteamPlayerBan>,
    comments: HashMap<SteamID, Vec<SteamProfileComment>>,
}

impl SteamApiCache {
    fn new() -> Self {
        Self {
            summaries: HashMap::new(),
            friends: HashMap::new(),
            playtimes: HashMap::new(),
            steam_bans: HashMap::new(),
            comments: HashMap::new(),
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

    fn get_playtime(&self, steamid: SteamID) -> Option<&Tf2PlayMinutes> {
        self.playtimes.get(&steamid)
    }

    fn set_playtime(&mut self, steamid: SteamID, playtime: &Tf2PlayMinutes) {
        self.playtimes.insert(steamid, playtime.clone());
    }

    fn get_steam_ban(&self, steamid: SteamID) -> Option<&SteamPlayerBan> {
        self.steam_bans.get(&steamid)
    }

    fn set_steam_ban(&mut self, steamid: SteamID, steam_ban: SteamPlayerBan) {
        self.steam_bans.insert(steamid, steam_ban);
    }

    fn get_comments(&self, steamid: SteamID) -> Option<&Vec<SteamProfileComment>> {
        self.comments.get(&steamid)
    }

    fn set_comments(&mut self, steamid: SteamID, comments: Vec<SteamProfileComment>) {
        self.comments.insert(steamid, comments);
    }
}

pub struct SteamApiThread {
    bus: Arc<Mutex<AppBus>>,
    shared_lobby: crate::tf2::lobby::shared_lobby::SharedLobby,
    steam_api: SteamApi,
    steam_api_cache: SteamApiCache,
    db: DatabaseConnection,
}

impl SteamApiThread {
    pub fn new(settings: &AppSettings, bus: &Arc<Mutex<AppBus>>, db: &DatabaseConnection) -> Self {
        let shared_lobby = bus.lock().unwrap().shared_lobby.clone();

        Self {
            bus: Arc::clone(bus),
            shared_lobby,
            steam_api: SteamApi::new(settings),
            steam_api_cache: SteamApiCache::new(),
            db: db.clone(),
        }
    }

    pub fn run(&mut self) {
        log::info!("SteamAPi background thread started");

        loop {
            self.get_latest_lobby();

            sleep(STEAMAPI_LOOP_DELAY);
        }
    }

    fn send(&mut self, msg: SteamApiMsg) {
        self.bus.lock().unwrap().steamapi_bus.broadcast(msg);
    }

    fn get_latest_lobby(&mut self) {
        // To fetch additional info from Steam Web Api a key is needed
        if !self.steam_api.has_key() {
            log::warn!("Steam API key is not set, skipping Steam API fetch");
            return;
        }

        // Get a copy of the current lobby state
        let lobby = self.shared_lobby.get();
        self.fetch_summaries(&lobby);
        self.fetch_steam_bans(&lobby);
        self.fetch_friends(&lobby);
        self.fetch_playtimes(&lobby);
        self.fetch_comments(&lobby);
        self.approximate_account_ages(&lobby);
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

        if let Ok(infos) = self.steam_api.get_player_summaries(summaries_to_fetch) {
            for info in infos {
                if let Some(steamid) = SteamID::from_u64_string(&info.steamid) {
                    let public_profile = matches!(info.communityvisibilitystate, 3);

                    let info = PlayerSteamInfo {
                        steamid,
                        public_profile,
                        // name: info.personaname.clone(),
                        avatar: info.avatar.clone(),
                        // avatarmedium: info.avatarmedium.clone(),
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
                self.send(SteamApiMsg::Tf2Playtime(steamid, playtime.clone()));
                continue;
            }

            // Not in cache, fetch from Steam API and put in cache
            log::info!("Fetching playtime for {}", player.name);
            let playtime = self.steam_api.get_tf2_play_minutes(steamid);
            self.steam_api_cache.set_playtime(steamid, &playtime);
            self.send(SteamApiMsg::Tf2Playtime(steamid, playtime));
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
            .filter(|p| p.tf2_play_minutes == Tf2PlayMinutes::Loading)
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
        const NEIGHBORHOOD_SIZE: u64 = 20;

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

        for _ in 0..5 {
            let accounts = self.steam_api.get_player_summaries(ids.clone());
            match accounts {
                Ok(accounts) => {
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
                Err(e) => log::error!("Error fetching player summaries: {}", e),
            }

            sleep(STEAMAPI_RETRY_DELAY);
        }

        log::info!("No neighbors with public profile found for {}", player.name);

        self.send(SteamApiMsg::ApproxAccountAge(
            player.steamid,
            AccountAge::Unknown,
        ));
    }

    fn fetch_comments(&mut self, lobby: &Lobby) {
        let mut comments_to_fetch = Vec::new();

        for player in lobby.players.iter() {
            if player.profile_comments.is_none() {
                // First check cache
                if let Some(comments) = self.steam_api_cache.get_comments(player.steamid) {
                    // log::info!("Fetched from cache ban of {}", player.name);
                    self.send(SteamApiMsg::ProfileComments(
                        player.steamid,
                        comments.clone(),
                    ));
                } else {
                    // Bulk fetch from Steam API below
                    comments_to_fetch.push(player.steamid);
                }
            }
        }

        for steamid in comments_to_fetch
            .into_iter()
            .take(NUM_PROFILE_COMMENTS_TO_FETCH)
        {
            if let Some(player) = lobby.get_player(None, Some(steamid)) {
                if player.steam_info.is_none() {
                    continue;
                }

                log::info!("Fetching profile comments for {}", player.name);

                let comments = get_steam_profile_comments(steamid.to_u64());
                if let Some(comments) = comments {
                    self.steam_api_cache.set_comments(steamid, comments.clone());
                    self.send(SteamApiMsg::ProfileComments(steamid, comments));
                } else {
                    log::info!("Error fetching comments for {}", steamid.to_u64());

                    // Set to empty comments to avoid fetching again
                    let comments = Vec::new();
                    self.steam_api_cache.set_comments(steamid, comments.clone());
                    self.send(SteamApiMsg::ProfileComments(steamid, comments));
                }
            }
        }
    }
}
