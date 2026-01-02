use super::{
    get_steam_comments::get_steam_profile_comments, SteamApi, SteamPlayerBan, SteamProfileComment,
};
use crate::config::{
    DB_CACHE_TTL_ACCOUNT_SECONDS, DB_CACHE_TTL_COMMENTS_SECONDS, DB_CACHE_TTL_FRIENDLIST_SECONDS,
    DB_CACHE_TTL_PLAYTIME_SECONDS, NUM_ACCOUNT_AGES_TO_APPROX, NUM_FRIENDS_TO_FETCH,
    NUM_PLAYTIMES_TO_FETCH, NUM_PROFILE_COMMENTS_TO_FETCH, STEAMAPI_LOOP_DELAY,
    STEAMAPI_RETRY_DELAY,
};
use crate::db::db::DbPool;
use crate::db::entities::{Game, NewAccount, NewComment, NewFriendship, NewPlaytime};
use crate::db::queries;
use crate::{
    appbus::AppBus,
    models::{app_settings::AppSettings, steamid::SteamID},
    tf2::{
        lobby::{AccountAge, Lobby, Player, PlayerSteamInfo, Tf2PlayMinutes},
        steamapi::SteamApiMsg,
    },
};
use chrono::Utc;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    thread::{self, sleep},
};

/// Start the background thread for the rcon module
pub fn start(
    settings: &AppSettings,
    bus: &Arc<Mutex<AppBus>>,
    db: &DbPool,
) -> thread::JoinHandle<()> {
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
    db: DbPool,
}

impl SteamApiThread {
    pub fn new(settings: &AppSettings, bus: &Arc<Mutex<AppBus>>, db: &DbPool) -> Self {
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
        let current_time = Utc::now().timestamp();

        for player in lobby.players.iter() {
            if player.steam_info.is_none() {
                // First check in-memory cache
                if let Some(summary) = self.steam_api_cache.get_summary(player.steamid) {
                    // log::info!("Fetched from in-memory cache summary of {}", player.name);
                    self.send(SteamApiMsg::PlayerSummary(summary.clone()));
                    continue;
                }

                // Check database
                if let Ok(mut conn) = self.db.get() {
                    if let Ok(Some(account)) =
                        queries::get_account_by_steam_id(&mut conn, player.steamid.to_u64() as i64)
                    {
                        let is_outdated =
                            current_time - account.last_updated > DB_CACHE_TTL_ACCOUNT_SECONDS;

                        // Convert database account to PlayerSteamInfo
                        let account_age = account.created_date.map(|ts| {
                            chrono::DateTime::from_timestamp(ts, 0)
                                .unwrap_or_else(|| chrono::DateTime::UNIX_EPOCH)
                                .with_timezone(&chrono::Local)
                        });

                        let summary = PlayerSteamInfo {
                            steamid: player.steamid,
                            public_profile: account.public_profile,
                            avatar_thumb: account.avatar_thumb_url.clone(),
                            avatar_full: account.avatar_full_url.clone(),
                            account_age,
                        };

                        // Add to in-memory cache
                        self.steam_api_cache.set_summary(summary.clone());

                        if is_outdated {
                            // Send cached data first so UI has something to show
                            log::info!(
                                "Sending outdated cached data for {}, will refresh",
                                player.name
                            );
                            self.send(SteamApiMsg::PlayerSummary(summary));
                            // Mark for refresh
                            summaries_to_fetch.push(player.steamid);
                        } else {
                            // Data is fresh, use it
                            // log::info!("Fetched from database summary of {}", player.name);
                            self.send(SteamApiMsg::PlayerSummary(summary));
                        }
                        continue;
                    }
                }

                // No cache hit, bulk fetch from Steam API below
                summaries_to_fetch.push(player.steamid);
            }
        }

        if !summaries_to_fetch.is_empty() {
            if let Ok(infos) = self.steam_api.get_player_summaries(summaries_to_fetch) {
                for info in infos {
                    if let Some(steamid) = SteamID::from_u64_string(&info.steamid) {
                        let public_profile = matches!(info.communityvisibilitystate, 3);
                        let account_age = info.get_account_age();

                        let info = PlayerSteamInfo {
                            steamid,
                            public_profile,
                            // name: info.personaname.clone(),
                            avatar_thumb: info.avatar.clone(),
                            // avatarmedium: info.avatarmedium.clone(),
                            avatar_full: info.avatarfull.clone(),
                            account_age: account_age.clone(),
                        };

                        // Add to cache and then send
                        self.steam_api_cache.set_summary(info.clone());
                        self.send(SteamApiMsg::PlayerSummary(info.clone()));

                        // Persist to database
                        if let Ok(mut conn) = self.db.get() {
                            // Find the player name from the lobby
                            let player_name = lobby
                                .get_player(None, Some(steamid))
                                .map(|p| p.name.clone())
                                .unwrap_or_else(|| String::from("Unknown"));

                            let created_date = account_age.map(|dt| dt.timestamp());

                            let new_account = NewAccount {
                                steam_id: steamid.to_u64() as i64,
                                name: player_name,
                                created_date,
                                avatar_thumb_url: info.avatar_thumb.clone(),
                                avatar_full_url: info.avatar_full.clone(),
                                public_profile: info.public_profile,
                                last_updated: Utc::now().timestamp(),
                                friends_fetched: None,
                                comments_fetched: None,
                                playtimes_fetched: None,
                                reputation_fetched: None,
                            };

                            if let Err(e) = queries::upsert_account(&mut conn, new_account) {
                                log::error!("Failed to persist account {}: {}", steamid.to_u64(), e);
                            }
                        }
                    }
                }
            }
        }
    }

    fn fetch_friends(&mut self, lobby: &Lobby) {
        let players = self.get_players_without_friends(lobby);
        let current_time = Utc::now().timestamp();

        for player in players {
            let steamid = player.steamid;

            if player.steam_info.is_some() {
                // First check in-memory cache
                if let Some(friends) = self.steam_api_cache.get_friends(steamid) {
                    // log::info!("Fetched from in-memory cache friends of {}", player.name);
                    self.send(SteamApiMsg::FriendsList(steamid, friends.clone()));
                    continue;
                }

                // Check database
                if let Ok(mut conn) = self.db.get() {
                    // Get account to check friends_fetched timestamp
                    if let Ok(Some(account)) =
                        queries::get_account_by_steam_id(&mut conn, steamid.to_u64() as i64)
                    {
                        if let Some(friends_fetched) = account.friends_fetched {
                            let is_outdated = current_time - friends_fetched
                                > DB_CACHE_TTL_FRIENDLIST_SECONDS;

                            // Get friendships from database
                            if let Ok(friendships) =
                                queries::get_friendships(&mut conn, steamid.to_u64() as i64, true)
                            {
                                // Convert to HashSet<SteamID>
                                let friends: HashSet<SteamID> = friendships
                                    .iter()
                                    .filter_map(|f| {
                                        // Check if this is a direct or reverse friendship
                                        if f.steam_id == steamid.to_u64() as i64 {
                                            Some(SteamID::from_u64(f.friend_steam_id as u64))
                                        } else {
                                            Some(SteamID::from_u64(f.steam_id as u64))
                                        }
                                    })
                                    .collect();

                                if !friends.is_empty() {
                                    // Add to in-memory cache
                                    self.steam_api_cache.set_friends(steamid, friends.clone());

                                    if is_outdated {
                                        // Send cached data first so UI has something to show
                                        log::info!(
                                            "Sending outdated cached friends for {}, will refresh",
                                            player.name
                                        );
                                        self.send(SteamApiMsg::FriendsList(steamid, friends));
                                        // Continue to fetch fresh data below
                                    } else {
                                        // Data is fresh, use it
                                        // log::info!("Fetched from database friends of {}", player.name);
                                        self.send(SteamApiMsg::FriendsList(steamid, friends));
                                        continue;
                                    }
                                }
                            }
                        }
                    }
                }

                // Not in cache or outdated, fetch from Steam API
                log::info!("Fetching friends of {}", player.name);
                if let Some(friends) = self.steam_api.get_friendlist(steamid) {
                    self.steam_api_cache.set_friends(steamid, friends.clone());
                    self.send(SteamApiMsg::FriendsList(steamid, friends.clone()));

                    // Persist to database
                    if let Ok(mut conn) = self.db.get() {
                        let current_time = Utc::now().timestamp();

                        // Insert/update each friendship
                        for friend_steamid in &friends {
                            // Get friend name from lobby if available
                            let friend_name = lobby
                                .get_player(None, Some(*friend_steamid))
                                .map(|p| p.name.clone())
                                .unwrap_or_else(|| String::from("Unknown"));

                            let new_friendship = NewFriendship {
                                steam_id: steamid.to_u64() as i64,
                                friend_steam_id: friend_steamid.to_u64() as i64,
                                friend_name,
                                friend_date: current_time,
                                unfriend_date: None,
                            };

                            if let Err(e) = queries::upsert_friendship(&mut conn, new_friendship) {
                                log::error!(
                                    "Failed to persist friendship {}->{}: {}",
                                    steamid.to_u64(),
                                    friend_steamid.to_u64(),
                                    e
                                );
                            }
                        }

                        // Update friends_fetched timestamp
                        if let Err(e) = queries::update_account_friends_fetched(
                            &mut conn,
                            steamid.to_u64() as i64,
                            current_time,
                        ) {
                            log::error!(
                                "Failed to update friends_fetched for {}: {}",
                                steamid.to_u64(),
                                e
                            );
                        }
                    }
                }
            }
        }
    }

    fn fetch_playtimes(&mut self, lobby: &Lobby) {
        let players = self.get_players_without_playtime(lobby);
        let current_time = Utc::now().timestamp();

        for player in players {
            let steamid = player.steamid;

            // First check in-memory cache
            if let Some(playtime) = self.steam_api_cache.get_playtime(steamid) {
                // log::info!("Fetched from in-memory cache playtime for {}", player.name);
                self.send(SteamApiMsg::Tf2Playtime(steamid, playtime.clone()));
                continue;
            }

            // Check database
            if let Ok(mut conn) = self.db.get() {
                // Get account to check playtimes_fetched timestamp
                if let Ok(Some(account)) =
                    queries::get_account_by_steam_id(&mut conn, steamid.to_u64() as i64)
                {
                    if let Some(playtimes_fetched) = account.playtimes_fetched {
                        let is_outdated = current_time - playtimes_fetched
                            > DB_CACHE_TTL_PLAYTIME_SECONDS;

                        // Get playtime from database
                        if let Ok(Some(playtime_record)) =
                            queries::get_playtime(&mut conn, steamid.to_u64() as i64, Game::Tf2)
                        {
                            let playtime = Tf2PlayMinutes::PlayMinutes(playtime_record.play_minutes as u32);

                            // Add to in-memory cache
                            self.steam_api_cache.set_playtime(steamid, &playtime);

                            if is_outdated {
                                // Send cached data first so UI has something to show
                                log::info!(
                                    "Sending outdated cached playtime for {}, will refresh",
                                    player.name
                                );
                                self.send(SteamApiMsg::Tf2Playtime(steamid, playtime));
                                // Continue to fetch fresh data below
                            } else {
                                // Data is fresh, use it
                                // log::info!("Fetched from database playtime for {}", player.name);
                                self.send(SteamApiMsg::Tf2Playtime(steamid, playtime));
                                continue;
                            }
                        }
                    }
                }
            }

            // Not in cache or outdated, fetch from Steam API
            log::info!("Fetching playtime for {}", player.name);
            let playtime = self.steam_api.get_tf2_play_minutes(steamid);
            self.steam_api_cache.set_playtime(steamid, &playtime);
            self.send(SteamApiMsg::Tf2Playtime(steamid, playtime.clone()));

            // Persist to database
            if let Tf2PlayMinutes::PlayMinutes(minutes) = playtime {
                if let Ok(mut conn) = self.db.get() {
                    let current_time = Utc::now().timestamp();

                    let new_playtime = NewPlaytime {
                        steam_id: steamid.to_u64() as i64,
                        game: Game::Tf2,
                        play_minutes: minutes as i64,
                        last_updated: current_time,
                    };

                    if let Err(e) = queries::upsert_playtime(&mut conn, new_playtime) {
                        log::error!("Failed to persist playtime for {}: {}", steamid.to_u64(), e);
                    }

                    // Update playtimes_fetched timestamp
                    if let Err(e) = queries::update_account_playtimes_fetched(
                        &mut conn,
                        steamid.to_u64() as i64,
                        current_time,
                    ) {
                        log::error!(
                            "Failed to update playtimes_fetched for {}: {}",
                            steamid.to_u64(),
                            e
                        );
                    }
                }
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
        let current_time = Utc::now().timestamp();

        for player in lobby.players.iter() {
            if player.profile_comments.is_none() {
                // First check in-memory cache
                if let Some(comments) = self.steam_api_cache.get_comments(player.steamid) {
                    // log::info!("Fetched from in-memory cache comments of {}", player.name);
                    self.send(SteamApiMsg::ProfileComments(
                        player.steamid,
                        comments.clone(),
                    ));
                    continue;
                }

                // Check database
                if let Ok(mut conn) = self.db.get() {
                    // Get account to check comments_fetched timestamp
                    if let Ok(Some(account)) = queries::get_account_by_steam_id(
                        &mut conn,
                        player.steamid.to_u64() as i64,
                    ) {
                        if let Some(comments_fetched) = account.comments_fetched {
                            let is_outdated = current_time - comments_fetched
                                > DB_CACHE_TTL_COMMENTS_SECONDS;

                            // Get comments from database
                            if let Ok(db_comments) = queries::get_active_comments_for_account(
                                &mut conn,
                                player.steamid.to_u64() as i64,
                            ) {
                                // Convert database comments to SteamProfileComment
                                let comments: Vec<SteamProfileComment> = db_comments
                                    .iter()
                                    .map(|c| SteamProfileComment {
                                        name: c.writer_name.clone(),
                                        steamid: SteamID::from_u64(c.writer_steam_id as u64),
                                        comment: c.comment.clone(),
                                    })
                                    .collect();

                                // Add to in-memory cache
                                self.steam_api_cache.set_comments(player.steamid, comments.clone());

                                if is_outdated {
                                    // Send cached data first so UI has something to show
                                    log::info!(
                                        "Sending outdated cached comments for {}, will refresh",
                                        player.name
                                    );
                                    self.send(SteamApiMsg::ProfileComments(player.steamid, comments));
                                    // Mark for refresh
                                    comments_to_fetch.push(player.steamid);
                                    continue;
                                } else {
                                    // Data is fresh, use it
                                    // log::info!("Fetched from database comments for {}", player.name);
                                    self.send(SteamApiMsg::ProfileComments(player.steamid, comments));
                                    continue;
                                }
                            }
                        }
                    }
                }

                // No cache hit, mark for fetch
                comments_to_fetch.push(player.steamid);
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
                    self.send(SteamApiMsg::ProfileComments(steamid, comments.clone()));

                    // Persist to database
                    if let Ok(mut conn) = self.db.get() {
                        let current_time = Utc::now().timestamp();

                        for comment in &comments {
                            let new_comment = NewComment {
                                steam_id: steamid.to_u64() as i64,
                                writer_steam_id: comment.steamid.to_u64() as i64,
                                writer_name: comment.name.clone(),
                                comment: comment.comment.clone(),
                                created_date: current_time,
                                deleted_date: None,
                            };

                            if let Err(e) = queries::insert_comment(&mut conn, new_comment) {
                                log::debug!("Comment already exists or error: {}", e);
                            }
                        }

                        // Update comments_fetched timestamp
                        if let Err(e) = queries::update_account_comments_fetched(
                            &mut conn,
                            steamid.to_u64() as i64,
                            current_time,
                        ) {
                            log::error!(
                                "Failed to update comments_fetched for {}: {}",
                                steamid.to_u64(),
                                e
                            );
                        }
                    }
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
