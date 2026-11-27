use super::{SteamApi, SteamProfileComment, get_steam_comments::get_steam_profile_comments};
use crate::config::{
    DB_CACHE_TTL_ACCOUNT_SECONDS, DB_CACHE_TTL_COMMENTS_SECONDS, DB_CACHE_TTL_FRIENDLIST_SECONDS,
    DB_CACHE_TTL_PLAYTIME_SECONDS, NUM_ACCOUNT_AGES_TO_APPROX, NUM_FRIENDS_TO_FETCH,
    NUM_PLAYTIMES_TO_FETCH, NUM_PROFILE_COMMENTS_TO_FETCH, STEAMAPI_LOOP_DELAY,
    STEAMAPI_RETRY_DELAY,
};
use crate::db::connection::DbPool;
use crate::db::entities::{Game, NewAccount, NewComment, NewFriendship, NewPlaytime};
use crate::db::queries;
use crate::{
    appbus::AppBus,
    models::{
        app_settings::AppSettings,
        player_profile::{FetchStatus, PlayerProfile},
        steamid::SteamID,
    },
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
    time::{Duration, Instant},
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

pub struct SteamApiThread {
    bus: Arc<Mutex<AppBus>>,
    shared_lobby: crate::tf2::lobby::shared_lobby::SharedLobby,
    steam_api: SteamApi,
    db: DbPool,
    profile_dirty: std::cell::RefCell<HashSet<SteamID>>,
    detail_targets: HashMap<SteamID, Player>,
    detail_started: HashMap<SteamID, Instant>,
    detail_attempted: HashMap<SteamID, Instant>,
    detail_status: HashMap<SteamID, FetchStatus>,
}

impl SteamApiThread {
    pub fn new(settings: &AppSettings, bus: &Arc<Mutex<AppBus>>, db: &DbPool) -> Self {
        let shared_lobby = bus.lock().unwrap().shared_lobby.clone();

        Self {
            bus: Arc::clone(bus),
            shared_lobby,
            steam_api: SteamApi::new(settings),
            db: db.clone(),
            profile_dirty: Default::default(),
            detail_targets: HashMap::new(),
            detail_started: HashMap::new(),
            detail_attempted: HashMap::new(),
            detail_status: HashMap::new(),
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
        // Background fetch targets are separate from real server participants.
        for target in self.detail_targets.values_mut() {
            match &msg {
                SteamApiMsg::PlayerSummary(info) if info.steamid == target.steamid => {
                    target.steam_info = Some(info.clone())
                }
                SteamApiMsg::FriendsList(id, value) if *id == target.steamid => {
                    target.friends = Some(value.clone())
                }
                SteamApiMsg::Tf2Playtime(id, value) if *id == target.steamid => {
                    target.tf2_play_minutes = value.clone()
                }
                SteamApiMsg::SteamBans(id, value) if *id == target.steamid => {
                    target.steam_bans = Some(value.clone())
                }
                SteamApiMsg::ProfileComments(id, value) if *id == target.steamid => {
                    target.profile_comments = Some(value.clone())
                }
                _ => {}
            }
        }
        self.bus.lock().unwrap().steamapi_bus.broadcast(msg);
    }

    fn detail_state(&mut self, id: SteamID, status: FetchStatus) {
        if self.detail_status.get(&id) != Some(&status) {
            self.detail_status.insert(id, status.clone());
            self.bus.lock().unwrap().profile_events.broadcast(
                crate::models::player_profile::ProfileEvent::FetchStatus(id, status),
            );
        }
    }

    fn persisted<T>(&self, id: SteamID, result: diesel::QueryResult<T>) -> diesel::QueryResult<T> {
        if result.is_ok() {
            self.profile_dirty.borrow_mut().insert(id);
            self.bus
                .lock()
                .unwrap()
                .profile_events
                .broadcast(crate::models::player_profile::ProfileEvent::Changed(id));
        } else {
            self.bus.lock().unwrap().profile_events.broadcast(
                crate::models::player_profile::ProfileEvent::FetchStatus(id, FetchStatus::Failed),
            );
        }
        result
    }

    fn get_latest_lobby(&mut self) {
        let requested = self.bus.lock().unwrap().profile_requests.clone();
        self.sync_profile_requests(&requested);
        if !self.steam_api.has_key() {
            for id in requested {
                self.detail_state(id, FetchStatus::Unavailable);
            }
            return;
        }

        let mut lobby = self.shared_lobby.get();
        self.include_requested_profiles(&mut lobby, &requested);
        self.fetch_summaries(&lobby);
        self.fetch_steam_bans(&lobby);
        self.fetch_friends(&lobby);
        self.fetch_playtimes(&lobby);
        self.fetch_comments(&lobby);
        self.approximate_account_ages(&lobby);
        self.update_profile_fetch_statuses(requested);
    }

    fn sync_profile_requests(&mut self, requested: &HashSet<SteamID>) {
        self.profile_dirty
            .borrow_mut()
            .retain(|id| requested.contains(id));
        self.detail_targets.retain(|id, _| requested.contains(id));
        self.detail_started.retain(|id, _| requested.contains(id));
        self.detail_attempted.retain(|id, _| requested.contains(id));
        self.detail_status.retain(|id, _| requested.contains(id));
        for &id in requested {
            self.detail_started.entry(id).or_insert_with(Instant::now);
            if !self.detail_targets.contains_key(&id) {
                let target = self.new_profile_target(id);
                self.detail_targets.insert(id, target);
            }
        }
    }

    fn new_profile_target(&self, id: SteamID) -> Player {
        let name = self
            .db
            .get()
            .ok()
            .and_then(|mut conn| {
                queries::get_account_by_steam_id(&mut conn, id.to_u64() as i64)
                    .ok()
                    .flatten()
            })
            .map_or_else(|| id.to_u64().to_string(), |account| account.name);
        Player {
            steamid: id,
            name,
            ..Default::default()
        }
    }

    /// Add offline profiles to the local fetch list and prioritize all requested profiles.
    fn include_requested_profiles(&mut self, lobby: &mut Lobby, requested: &HashSet<SteamID>) {
        for &id in requested {
            if !self.detail_status.contains_key(&id) {
                self.detail_state(id, FetchStatus::Loading);
            }
            if lobby.get_player(None, Some(id)).is_none()
                && self
                    .detail_attempted
                    .get(&id)
                    .is_none_or(|at| at.elapsed() >= Duration::from_secs(5))
            {
                lobby.players.push(self.detail_targets[&id].clone());
                self.detail_attempted.insert(id, Instant::now());
            }
        }
        lobby
            .players
            .sort_by_key(|player| !requested.contains(&player.steamid));
    }

    fn update_profile_fetch_statuses(&mut self, requested: HashSet<SteamID>) {
        for id in requested {
            let lobby = self.shared_lobby.get();
            let player = lobby
                .get_player(None, Some(id))
                .unwrap_or_else(|| &self.detail_targets[&id]);
            let received_profile = player.steam_info.is_some()
                && player.steam_bans.is_some()
                && player.friends.is_some()
                && player.tf2_play_minutes != Tf2PlayMinutes::Loading
                && player.profile_comments.is_some();
            let changed = self.profile_dirty.borrow_mut().remove(&id);
            if received_profile {
                if changed || self.detail_status.get(&id) == Some(&FetchStatus::Loading) {
                    let status = if self.cached_profile_is_complete(id) {
                        FetchStatus::Ready
                    } else {
                        FetchStatus::Failed
                    };
                    self.detail_state(id, status);
                }
            } else if self.detail_started[&id].elapsed() > Duration::from_secs(30) {
                self.detail_state(id, FetchStatus::Failed);
            }
        }
    }

    /// Responses can arrive before writes finish, so readiness depends on the cache.
    fn cached_profile_is_complete(&self, id: SteamID) -> bool {
        self.db
            .get()
            .ok()
            .and_then(|mut conn| PlayerProfile::load(&mut conn, id).ok())
            .is_some_and(|profile| {
                profile.steam_info.is_some()
                    && profile.bans.is_some()
                    && profile.friends.is_some()
                    && profile.comments.is_some()
                    && profile.playtime != Tf2PlayMinutes::Loading
            })
    }

    fn fetch_summaries(&mut self, lobby: &Lobby) {
        let mut summaries_to_fetch = Vec::new();
        let current_time = Utc::now().timestamp();

        for player in lobby.players.iter() {
            if player.steam_info.is_none() {
                // Check database
                if let Ok(mut conn) = self.db.get()
                    && let Ok(Some(account)) =
                        queries::get_account_by_steam_id(&mut conn, player.steamid.to_u64() as i64)
                    && account.last_updated > 0
                {
                    let is_outdated =
                        current_time - account.last_updated > DB_CACHE_TTL_ACCOUNT_SECONDS;

                    // Convert database account to PlayerSteamInfo
                    let account_age = account.created_date.map(|ts| {
                        chrono::DateTime::from_timestamp(ts, 0)
                            .unwrap_or(chrono::DateTime::UNIX_EPOCH)
                            .with_timezone(&chrono::Local)
                    });

                    let summary = PlayerSteamInfo {
                        steamid: player.steamid,
                        public_profile: account.public_profile,
                        avatar_thumb: account.avatar_thumb_url.clone(),
                        avatar_full: account.avatar_full_url.clone(),
                        account_age,
                    };

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

                // No cache hit, bulk fetch from Steam API below
                summaries_to_fetch.push(player.steamid);
            }
        }

        if !summaries_to_fetch.is_empty()
            && let Ok(infos) = self.steam_api.get_player_summaries(summaries_to_fetch)
        {
            for info in infos {
                if let Some(steamid) = SteamID::from_u64_string(&info.steamid) {
                    let fetched_name = if info.personaname.is_empty() {
                        lobby
                            .get_player(None, Some(steamid))
                            .map_or_else(|| steamid.to_u64().to_string(), |p| p.name.clone())
                    } else {
                        info.personaname.clone()
                    };
                    let public_profile = matches!(info.communityvisibilitystate, 3);
                    let account_age = info.get_account_age();

                    let info = PlayerSteamInfo {
                        steamid,
                        public_profile,
                        // name: info.personaname.clone(),
                        avatar_thumb: info.avatar.clone(),
                        // avatarmedium: info.avatarmedium.clone(),
                        avatar_full: info.avatarfull.clone(),
                        account_age,
                    };

                    // Send to lobby
                    self.send(SteamApiMsg::PlayerSummary(info.clone()));

                    // Persist to database
                    if let Ok(mut conn) = self.db.get() {
                        let created_date = account_age.map(|dt| dt.timestamp());

                        let new_account = NewAccount {
                            steam_id: steamid.to_u64() as i64,
                            name: fetched_name,
                            created_date,
                            avatar_thumb_url: info.avatar_thumb.clone(),
                            avatar_full_url: info.avatar_full.clone(),
                            public_profile: info.public_profile,
                            last_updated: Utc::now().timestamp(),
                            friends_fetched: None,
                            comments_fetched: None,
                            playtimes_fetched: None,
                            steam_bans_last_fetched: None,
                        };

                        if let Err(e) =
                            self.persisted(steamid, queries::upsert_account(&mut conn, new_account))
                        {
                            log::error!("Failed to persist account {}: {}", steamid.to_u64(), e);
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
                // Check database
                if let Ok(mut conn) = self.db.get() {
                    // Get friendships from database first
                    if let Ok(friendships) =
                        queries::get_friendships(&mut conn, steamid.to_u64() as i64, false)
                    {
                        // Convert to HashSet<SteamID>
                        let friends: HashSet<SteamID> = friendships
                            .iter()
                            .map(|f| {
                                // Check if this is a direct or reverse friendship
                                if f.steam_id == steamid.to_u64() as i64 {
                                    SteamID::from_u64(f.friend_steam_id as u64)
                                } else {
                                    SteamID::from_u64(f.steam_id as u64)
                                }
                            })
                            .collect();

                        // Check if we need to refresh by looking at the account's friends_fetched timestamp
                        // This check should happen even if friends list is empty (player might have 0 friends)
                        let should_refresh = if let Ok(Some(account)) =
                            queries::get_account_by_steam_id(&mut conn, steamid.to_u64() as i64)
                        {
                            // Only refresh if timestamp exists AND is outdated
                            account
                                .friends_fetched
                                .map(|ts| current_time - ts > DB_CACHE_TTL_FRIENDLIST_SECONDS)
                                .unwrap_or(true) // If no timestamp exists, fetch from API
                        } else {
                            true // If no account exists, fetch from API
                        };

                        if should_refresh {
                            // If friends list is not empty, send cached data first so UI has something to show
                            if !friends.is_empty() {
                                log::info!(
                                    "Sending outdated cached friends for {}, will refresh",
                                    player.name
                                );
                                self.send(SteamApiMsg::FriendsList(steamid, friends));
                            }
                            // Continue to fetch fresh data below
                        } else {
                            // Data is fresh, use cached data (even if empty)
                            // log::info!("Fetched from database friends of {}", player.name);
                            self.send(SteamApiMsg::FriendsList(steamid, friends));
                            continue;
                        }
                    }
                }

                // Not in cache or outdated, fetch from Steam API
                log::info!("Fetching friends of {}", player.name);
                if let Some(friends) = self.steam_api.get_friendlist(steamid) {
                    let mut display_friends = friends.clone();

                    // Preserve known friendships in the UI even if the live profile no longer
                    // exposes them, so players cannot hide old relationships by clearing their
                    // friend list after we have already observed it.
                    if let Ok(mut conn) = self.db.get()
                        && let Ok(friendships) =
                            queries::get_friendships(&mut conn, steamid.to_u64() as i64, false)
                    {
                        display_friends.extend(friendships.iter().map(|f| {
                            if f.steam_id == steamid.to_u64() as i64 {
                                SteamID::from_u64(f.friend_steam_id as u64)
                            } else {
                                SteamID::from_u64(f.steam_id as u64)
                            }
                        }));
                    }

                    self.send(SteamApiMsg::FriendsList(steamid, display_friends));

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

                            if let Err(e) = self.persisted(
                                *friend_steamid,
                                self.persisted(
                                    steamid,
                                    queries::upsert_friendship(&mut conn, new_friendship),
                                ),
                            ) {
                                log::error!(
                                    "Failed to persist friendship {}->{}: {}",
                                    steamid.to_u64(),
                                    friend_steamid.to_u64(),
                                    e
                                );
                            }
                        }

                        // Update friends_fetched timestamp
                        if let Err(e) = self.persisted(
                            steamid,
                            queries::update_account_friends_fetched(
                                &mut conn,
                                steamid.to_u64() as i64,
                                current_time,
                            ),
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

            // Check database
            if let Ok(mut conn) = self.db.get() {
                // Get playtime from database first
                if let Ok(Some(playtime_record)) =
                    queries::get_playtime(&mut conn, steamid.to_u64() as i64, Game::Tf2)
                {
                    let playtime = match playtime_record.play_minutes {
                        Some(minutes) => {
                            log::info!(
                                "Found playtime in database for {} ({}): {} minutes",
                                player.name,
                                steamid.to_u64(),
                                minutes
                            );
                            Tf2PlayMinutes::PlayMinutes(minutes as u32)
                        }
                        None => {
                            log::info!(
                                "Found Unknown playtime in database for {} ({})",
                                player.name,
                                steamid.to_u64()
                            );
                            Tf2PlayMinutes::Unknown
                        }
                    };

                    // Check if we need to refresh by looking at the account's playtimes_fetched timestamp
                    let should_refresh = if let Ok(Some(account)) =
                        queries::get_account_by_steam_id(&mut conn, steamid.to_u64() as i64)
                    {
                        // Only refresh if timestamp exists AND is outdated

                        account
                            .playtimes_fetched
                            .map(|ts| {
                                let age = current_time - ts;
                                let is_outdated = age > DB_CACHE_TTL_PLAYTIME_SECONDS;
                                log::info!(
                                    "Playtime timestamp for {}: age={}s, outdated={}",
                                    player.name,
                                    age,
                                    is_outdated
                                );
                                is_outdated
                            })
                            .unwrap_or_else(|| {
                                log::info!(
                                    "Playtime timestamp for {} is None, not refreshing",
                                    player.name
                                );
                                false
                            })
                    } else {
                        log::info!("No account record for {}, not refreshing", player.name);
                        false
                    };

                    if should_refresh {
                        // Send cached data first so UI has something to show
                        log::info!(
                            "Sending outdated cached playtime for {}, will refresh",
                            player.name
                        );
                        self.send(SteamApiMsg::Tf2Playtime(steamid, playtime));
                        // Continue to fetch fresh data below
                    } else {
                        // Data is fresh or no timestamp, use cached data
                        log::info!("Using cached playtime from database for {}", player.name);
                        self.send(SteamApiMsg::Tf2Playtime(steamid, playtime));
                        continue;
                    }
                } else {
                    log::info!(
                        "No playtime found in database for {}, will fetch from API",
                        player.name
                    );
                }
            }

            // Not in cache or outdated, fetch from Steam API
            log::info!("Fetching playtime for {}", player.name);
            let playtime = self.steam_api.get_tf2_play_minutes(steamid);
            self.send(SteamApiMsg::Tf2Playtime(steamid, playtime.clone()));

            // Persist to database
            // Save both PlayMinutes and Unknown to avoid re-fetching every time
            match &playtime {
                Tf2PlayMinutes::PlayMinutes(minutes) => {
                    if let Ok(mut conn) = self.db.get() {
                        let current_time = Utc::now().timestamp();

                        let new_playtime = NewPlaytime {
                            steam_id: steamid.to_u64() as i64,
                            game: Game::Tf2,
                            play_minutes: Some(*minutes as i64),
                            last_updated: current_time,
                        };

                        if let Err(e) = self
                            .persisted(steamid, queries::upsert_playtime(&mut conn, new_playtime))
                        {
                            log::error!(
                                "Failed to persist playtime for {}: {}",
                                steamid.to_u64(),
                                e
                            );
                        }

                        // Update playtimes_fetched timestamp
                        if let Err(e) = self.persisted(
                            steamid,
                            queries::update_account_playtimes_fetched(
                                &mut conn,
                                steamid.to_u64() as i64,
                                current_time,
                            ),
                        ) {
                            log::error!(
                                "Failed to update playtimes_fetched for {}: {}",
                                steamid.to_u64(),
                                e
                            );
                        } else {
                            log::info!(
                                "Updated playtimes_fetched timestamp for {}",
                                steamid.to_u64()
                            );
                        }
                    }
                }
                Tf2PlayMinutes::Unknown => {
                    // For Unknown playtime, store NULL to avoid re-fetching
                    if let Ok(mut conn) = self.db.get() {
                        let current_time = Utc::now().timestamp();

                        let new_playtime = NewPlaytime {
                            steam_id: steamid.to_u64() as i64,
                            game: Game::Tf2,
                            play_minutes: None,
                            last_updated: current_time,
                        };

                        if let Err(e) = self
                            .persisted(steamid, queries::upsert_playtime(&mut conn, new_playtime))
                        {
                            log::error!(
                                "Failed to persist playtime for {}: {}",
                                steamid.to_u64(),
                                e
                            );
                        }

                        // Update playtimes_fetched timestamp
                        if let Err(e) = self.persisted(
                            steamid,
                            queries::update_account_playtimes_fetched(
                                &mut conn,
                                steamid.to_u64() as i64,
                                current_time,
                            ),
                        ) {
                            log::error!(
                                "Failed to update playtimes_fetched for {}: {}",
                                steamid.to_u64(),
                                e
                            );
                        } else {
                            log::info!(
                                "Updated playtimes_fetched timestamp for {} (Unknown playtime)",
                                steamid.to_u64()
                            );
                        }
                    }
                }
                Tf2PlayMinutes::Loading => {
                    // Don't save Loading state
                }
            }
        }
    }

    fn fetch_steam_bans(&mut self, lobby: &Lobby) {
        let mut bans_to_fetch = Vec::new();

        for player in lobby.players.iter() {
            if player.steam_bans.is_none() {
                // Bulk fetch from Steam API below
                bans_to_fetch.push(player.steamid);
            }
        }

        if bans_to_fetch.is_empty() {
            return;
        }
        if let Some(bans) = self.steam_api.get_bans(bans_to_fetch) {
            let current_time = Utc::now().timestamp();

            for ban in bans {
                // Send to lobby
                self.send(SteamApiMsg::SteamBans(ban.steamid, ban.clone()));

                // Persist to database
                if let Ok(mut conn) = self.db.get() {
                    use crate::db::entities::NewSteamBan;

                    let new_ban = NewSteamBan {
                        steam_id: ban.steamid.to_u64() as i64,
                        community_banned: ban.community_banned,
                        vac_banned: ban.vac_banned,
                        number_of_vac_bans: ban.number_of_vac_bans as i32,
                        days_since_last_ban: ban.days_since_last_ban as i32,
                        number_of_game_bans: ban.number_of_game_bans as i32,
                        economy_ban: String::from("none"), // Default value, Steam API doesn't provide this in our struct
                    };

                    if let Err(e) =
                        self.persisted(ban.steamid, queries::upsert_steam_bans(&mut conn, &new_ban))
                    {
                        log::error!(
                            "Failed to persist steam bans for {}: {}",
                            ban.steamid.to_u64(),
                            e
                        );
                    } else {
                        // Update the steam_bans_last_fetched timestamp for this account
                        if let Err(e) = self.persisted(
                            ban.steamid,
                            queries::update_steam_bans_last_fetched(
                                &mut conn,
                                ban.steamid.to_u64() as i64,
                                current_time,
                            ),
                        ) {
                            log::debug!(
                                "Failed to update steam_bans_last_fetched for {}: {}",
                                ban.steamid.to_u64(),
                                e
                            );
                        }
                    }
                }
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
                // Check database
                if let Ok(mut conn) = self.db.get() {
                    // Get comments from database first
                    if let Ok(db_comments) = queries::get_active_comments_for_account(
                        &mut conn,
                        player.steamid.to_u64() as i64,
                    ) {
                        // Convert database comments to SteamProfileComment
                        let comments: Vec<SteamProfileComment> = db_comments
                            .iter()
                            .map(|c| SteamProfileComment {
                                name: c.writer_name.clone(),
                                avatar_thumb: c.writer_avatar_thumb.clone(),
                                steamid: SteamID::from_u64(c.writer_steam_id as u64),
                                comment: c.comment.clone(),
                            })
                            .collect();

                        // Check if we need to refresh by looking at the account's comments_fetched timestamp
                        let should_refresh = if let Ok(Some(account)) =
                            queries::get_account_by_steam_id(
                                &mut conn,
                                player.steamid.to_u64() as i64,
                            ) {
                            // No timestamp means we have never fetched comments for this profile,
                            // even if the database currently has zero cached rows.
                            account
                                .comments_fetched
                                .map(|ts| current_time - ts > DB_CACHE_TTL_COMMENTS_SECONDS)
                                .unwrap_or(true)
                        } else {
                            true
                        };

                        // Populate thumbnails for comments cached before avatar support.
                        let missing_avatars = comments.iter().any(|c| c.avatar_thumb.is_empty());
                        let should_refresh =
                            DB_CACHE_TTL_COMMENTS_SECONDS <= 0 || should_refresh || missing_avatars;
                        if should_refresh {
                            // Only send cached comments if we actually have some.
                            // An empty DB result is not a real cache hit and should not mark
                            // the player as "done", otherwise only the first few queued
                            // profiles ever get a live fetch.
                            if !comments.is_empty() && !missing_avatars {
                                log::info!(
                                    "Sending outdated cached comments for {}, will refresh",
                                    player.name
                                );
                                self.send(SteamApiMsg::ProfileComments(player.steamid, comments));
                            }
                            // Mark for refresh
                            comments_to_fetch.push(player.steamid);
                            continue;
                        } else {
                            // Data is fresh or no timestamp, use cached data
                            log::info!("Using cached comments from database for {}", player.name);
                            self.send(SteamApiMsg::ProfileComments(player.steamid, comments));
                            continue;
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
                log::info!("Fetching profile comments for {}", player.name);

                let comments = get_steam_profile_comments(steamid.to_u64());
                if let Some(comments) = comments {
                    self.send(SteamApiMsg::ProfileComments(steamid, comments.clone()));

                    // Persist to database
                    if let Ok(mut conn) = self.db.get() {
                        let current_time = Utc::now().timestamp();

                        for comment in &comments {
                            let new_comment = NewComment {
                                steam_id: steamid.to_u64() as i64,
                                writer_steam_id: comment.steamid.to_u64() as i64,
                                writer_name: comment.name.clone(),
                                writer_avatar_thumb: comment.avatar_thumb.clone(),
                                comment: comment.comment.clone(),
                                created_date: current_time,
                                deleted_date: None,
                            };

                            if let Err(e) = self
                                .persisted(steamid, queries::insert_comment(&mut conn, new_comment))
                            {
                                log::debug!("Comment already exists or error: {}", e);
                            }
                        }

                        // Update comments_fetched timestamp
                        if let Err(e) = self.persisted(
                            steamid,
                            queries::update_account_comments_fetched(
                                &mut conn,
                                steamid.to_u64() as i64,
                                current_time,
                            ),
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
                    self.send(SteamApiMsg::ProfileComments(steamid, comments));
                }
            }
        }
    }
}

#[cfg(test)]
mod profile_tests {
    use super::*;
    use crate::models::player_profile::{FetchStatus, ProfileEvent, seed_account, test_app};

    #[test]
    fn open_database_profiles_become_fetch_targets_without_becoming_lobby_players() {
        let mut app = test_app();
        let id = SteamID::from_u64(crate::models::steamid::MIN_STEAMID64 + 1);
        seed_account(&app, id, "Offline profile");
        app.open_player_details_window(id);
        app.get_latest_lobby();
        let mut settings = app.app_settings.clone();
        settings.steam_api_key.clear();
        let mut worker = SteamApiThread::new(&settings, &app.bus, &app.db);
        worker.get_latest_lobby(); // No key: must not perform network requests.
        assert!(worker.detail_targets.contains_key(&id));
        assert!(app.shared_lobby.get().players.is_empty());
        worker.send(SteamApiMsg::Tf2Playtime(
            id,
            Tf2PlayMinutes::PlayMinutes(600),
        ));
        assert_eq!(
            worker.detail_targets[&id].tf2_play_minutes,
            Tf2PlayMinutes::PlayMinutes(600)
        );
        assert!(app.shared_lobby.get().players.is_empty());
        app.get_latest_lobby();
        assert_eq!(app.profile_status[&id], FetchStatus::Unavailable);
        app.open_player_details_windows.clear();
        app.get_latest_lobby();
        worker.get_latest_lobby();
        assert!(!worker.detail_targets.contains_key(&id));
    }

    #[test]
    fn profile_notifications_follow_successful_database_writes_only() {
        let app = test_app();
        let id = SteamID::from_u64(crate::models::steamid::MIN_STEAMID64 + 1);
        seed_account(&app, id, "Profile");
        let worker = SteamApiThread::new(&app.app_settings, &app.bus, &app.db);
        let mut events = app.bus.lock().unwrap().profile_events.add_rx();
        let mut conn = app.db.get().unwrap();
        worker
            .persisted(
                id,
                queries::update_account_comments_fetched(&mut conn, id.to_u64() as i64, 123),
            )
            .unwrap();
        assert!(matches!(events.try_recv().unwrap(), ProfileEvent::Changed(value) if value == id));
        assert_eq!(
            queries::get_account_by_steam_id(&mut conn, id.to_u64() as i64)
                .unwrap()
                .unwrap()
                .comments_fetched,
            Some(123)
        );
        let failure: diesel::QueryResult<()> = Err(diesel::result::Error::RollbackTransaction);
        assert!(worker.persisted(id, failure).is_err());
        assert!(
            matches!(events.try_recv().unwrap(), ProfileEvent::FetchStatus(value, FetchStatus::Failed) if value == id)
        );
        assert!(events.try_recv().is_err());
    }
}
