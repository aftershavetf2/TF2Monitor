//! Persistent player information, independent of membership of a game server.
use crate::{
    db::{entities::Game, queries, schema},
    models::steamid::SteamID,
    tf2::{
        lobby::{Player, PlayerSteamInfo, Tf2PlayMinutes},
        steamapi::{SteamPlayerBan, SteamProfileComment},
    },
    tf2bd::models::{PlayerAttribute, PlayerInfo},
};
use diesel::prelude::*;
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct PlayerProfile {
    pub steamid: SteamID,
    pub name: String,
    pub steam_info: Option<PlayerSteamInfo>,
    pub friends: Option<HashSet<SteamID>>,
    pub playtime: Tf2PlayMinutes,
    pub bans: Option<SteamPlayerBan>,
    pub comments: Option<Vec<SteamProfileComment>>,
    pub flags: Vec<PlayerAttribute>,
}

impl PlayerProfile {
    pub fn load(conn: &mut SqliteConnection, steamid: SteamID) -> QueryResult<Self> {
        conn.transaction(|conn| {
            let id = steamid.to_u64() as i64;
            let account = queries::get_account_by_steam_id(conn, id)?;
            let steam_info =
                account
                    .as_ref()
                    .filter(|a| a.last_updated > 0)
                    .map(|a| PlayerSteamInfo {
                        steamid,
                        public_profile: a.public_profile,
                        avatar_thumb: a.avatar_thumb_url.clone(),
                        avatar_full: a.avatar_full_url.clone(),
                        account_age: a
                            .created_date
                            .and_then(chrono::DateTime::from_timestamp_secs)
                            .map(|d| d.with_timezone(&chrono::Local)),
                    });
            let friendships = queries::get_friendships(conn, id, false)?;
            let friends = if account
                .as_ref()
                .is_some_and(|a| a.friends_fetched.is_some())
                || !friendships.is_empty()
            {
                Some(
                    friendships
                        .into_iter()
                        .map(|f| {
                            SteamID::from_u64(if f.steam_id == id {
                                f.friend_steam_id
                            } else {
                                f.steam_id
                            } as u64)
                        })
                        .collect(),
                )
            } else {
                None
            };
            let playtime = match queries::get_playtime(conn, id, Game::Tf2)? {
                Some(p) => p.play_minutes.map_or(Tf2PlayMinutes::Unknown, |m| {
                    Tf2PlayMinutes::PlayMinutes(m.max(0) as u32)
                }),
                None => Tf2PlayMinutes::Loading,
            };
            let bans = schema::steam_bans::table
                .find(id)
                .select((
                    schema::steam_bans::community_banned,
                    schema::steam_bans::vac_banned,
                    schema::steam_bans::number_of_vac_bans,
                    schema::steam_bans::days_since_last_ban,
                    schema::steam_bans::number_of_game_bans,
                ))
                .first::<(bool, bool, i32, i32, i32)>(conn)
                .optional()?
                .map(
                    |(community_banned, vac_banned, vac, days, games)| SteamPlayerBan {
                        steamid,
                        community_banned,
                        vac_banned,
                        number_of_vac_bans: vac as u32,
                        days_since_last_ban: days as u32,
                        number_of_game_bans: games as u32,
                    },
                );
            let rows = queries::get_active_comments_for_account(conn, id)?;
            let comments = if account
                .as_ref()
                .is_some_and(|a| a.comments_fetched.is_some())
                || !rows.is_empty()
            {
                Some(
                    rows.into_iter()
                        .map(|c| SteamProfileComment {
                            steamid: SteamID::from_u64(c.writer_steam_id as u64),
                            name: c.writer_name,
                            avatar_thumb: c.writer_avatar_thumb,
                            comment: c.comment,
                        })
                        .collect(),
                )
            } else {
                None
            };
            let flags = schema::player_flags::table
                .filter(schema::player_flags::steam_id.eq(id))
                .select(schema::player_flags::flag_type)
                .distinct()
                .load::<String>(conn)?
                .into_iter()
                .filter_map(|f| match f.as_str() {
                    "Cheater" => Some(PlayerAttribute::Cheater),
                    "Bot" => Some(PlayerAttribute::Bot),
                    "Cool" => Some(PlayerAttribute::Cool),
                    "Suspicious" => Some(PlayerAttribute::Suspicious),
                    "Exploiter" => Some(PlayerAttribute::Exploiter),
                    "Toxic" | "Racist" => Some(PlayerAttribute::Toxic),
                    _ => None,
                })
                .collect();
            Ok(Self {
                steamid,
                name: account.map_or_else(|| steamid.to_u64().to_string(), |a| a.name),
                steam_info,
                friends,
                playtime,
                bans,
                comments,
                flags,
            })
        })
    }

    /// Adapter for existing presentation helpers; never inserted into the lobby.
    pub fn display_player(&self) -> Player {
        Player {
            steamid: self.steamid,
            name: self.name.clone(),
            steam_info: self.steam_info.clone(),
            friends: self.friends.clone(),
            tf2_play_minutes: self.playtime.clone(),
            steam_bans: self.bans.clone(),
            profile_comments: self.comments.clone(),
            player_info: Some(PlayerInfo {
                steamid32: self.steamid.to_steam_id32(),
                attributes: self.flags.clone(),
                last_seen: None,
            }),
            ..Default::default()
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FetchStatus {
    Loading,
    Ready,
    Unavailable,
    Failed,
}

#[derive(Clone, Debug)]
pub enum ProfileEvent {
    Changed(SteamID),
    FetchStatus(SteamID, FetchStatus),
    Reputation(crate::reputation::Reputation),
}

/// Only game-session information; persistent profile data never belongs here.
#[derive(Clone, Debug)]
pub struct MatchSnapshot {
    pub session_id: u64,
    pub kills: u32,
    pub deaths: u32,
    pub team: crate::tf2::lobby::Team,
    pub ping_ms: i64,
    pub health: Option<u32>,
    pub kills_with: Vec<crate::tf2::lobby::PlayerKill>,
}
impl MatchSnapshot {
    pub fn new(session_id: u64, player: &Player) -> Self {
        Self {
            session_id,
            kills: player.kills,
            deaths: player.deaths,
            team: player.team,
            ping_ms: player.ping_ms,
            health: player.health,
            kills_with: player.kills_with.clone(),
        }
    }
}

#[cfg(test)]
pub(crate) fn test_app() -> crate::models::AppWin {
    use diesel::r2d2::{ConnectionManager, Pool};
    use std::sync::{Arc, Mutex};
    let settings = crate::models::app_settings::AppSettings::default();
    let bus = Arc::new(Mutex::new(crate::appbus::AppBus::new(
        settings.self_steamid64,
    )));
    let pool = Pool::builder()
        .max_size(1)
        .build(ConnectionManager::<SqliteConnection>::new(":memory:"))
        .unwrap();
    crate::db::connection::setup_schema(&mut pool.get().unwrap()).unwrap();
    crate::models::AppWin::new(&settings, &bus, Arc::new(pool))
}

#[cfg(test)]
pub(crate) fn seed_account(app: &crate::models::AppWin, id: SteamID, name: &str) {
    diesel::sql_query("INSERT INTO account (steam_id, name, avatar_thumb_url, avatar_full_url, public_profile, last_updated) VALUES (?, ?, '', '', 0, 0)")
        .bind::<diesel::sql_types::BigInt, _>(id.to_u64() as i64)
        .bind::<diesel::sql_types::Text, _>(name).execute(&mut app.db.get().unwrap()).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{db::entities::NewPlaytime, tf2::lobby::Player};
    fn id(n: u64) -> SteamID {
        SteamID::from_u64(crate::models::steamid::MIN_STEAMID64 + n)
    }

    #[test]
    fn profiles_distinguish_missing_data_from_confirmed_empty_and_private_data() {
        let app = test_app();
        seed_account(&app, id(1), "Offline player");
        let mut conn = app.db.get().unwrap();
        let profile = PlayerProfile::load(&mut conn, id(1)).unwrap();
        assert_eq!(profile.name, "Offline player");
        assert!(profile.steam_info.is_none()); // A placeholder is not a fetched private profile.
        assert!(profile.friends.is_none());
        assert!(profile.comments.is_none());
        assert!(profile.bans.is_none());
        assert_eq!(profile.playtime, Tf2PlayMinutes::Loading);
        queries::update_account_friends_fetched(&mut conn, id(1).to_u64() as i64, 100).unwrap();
        queries::update_account_comments_fetched(&mut conn, id(1).to_u64() as i64, 100).unwrap();
        queries::upsert_playtime(
            &mut conn,
            NewPlaytime {
                steam_id: id(1).to_u64() as i64,
                game: Game::Tf2,
                play_minutes: None,
                last_updated: 100,
            },
        )
        .unwrap();
        let profile = PlayerProfile::load(&mut conn, id(1)).unwrap();
        assert!(profile.friends.unwrap().is_empty());
        assert!(profile.comments.unwrap().is_empty());
        assert_eq!(profile.playtime, Tf2PlayMinutes::Unknown);
    }

    #[test]
    fn committed_events_invalidate_only_the_changed_profile() {
        let mut app = test_app();
        for n in [1, 2] {
            seed_account(&app, id(n), "Original");
            app.player_profile(id(n)).unwrap();
        }
        diesel::update(schema::account::table.find(id(1).to_u64() as i64))
            .set(schema::account::name.eq("Updated"))
            .execute(&mut app.db.get().unwrap())
            .unwrap();
        // Rendering does not poll the database on every frame.
        assert_eq!(app.player_profile(id(1)).unwrap().name, "Original");
        app.bus
            .lock()
            .unwrap()
            .profile_events
            .broadcast(ProfileEvent::Changed(id(1)));
        app.get_latest_lobby();
        assert!(!app.profiles.borrow().contains_key(&id(1)));
        assert!(app.profiles.borrow().contains_key(&id(2)));
        assert_eq!(app.player_profile(id(1)).unwrap().name, "Updated");
    }

    #[test]
    fn leaving_preserves_session_stats_but_server_switch_clears_them_and_keeps_profiles() {
        let mut app = test_app();
        seed_account(&app, id(1), "Player");
        app.player_profile(id(1)).unwrap();
        let mut lobby = app.shared_lobby.get();
        lobby.players.push(Player {
            steamid: id(1),
            kills: 12,
            deaths: 3,
            id: 17,
            ..Default::default()
        });
        app.shared_lobby.set(lobby.clone());
        app.get_latest_lobby();
        assert_eq!(app.match_snapshots[&id(1)].kills, 12);
        lobby.players.clear();
        app.shared_lobby.set(lobby.clone());
        app.get_latest_lobby();
        assert_eq!(app.match_snapshots[&id(1)].kills, 12);
        lobby.session_id += 1;
        app.shared_lobby.set(lobby.clone());
        app.get_latest_lobby();
        assert!(app.match_snapshots.is_empty());
        assert_eq!(app.player_profile(id(1)).unwrap().name, "Player");
        lobby.players.push(Player {
            steamid: id(1),
            kills: 1,
            id: 5,
            ..Default::default()
        });
        app.shared_lobby.set(lobby);
        app.get_latest_lobby();
        assert_eq!(app.match_snapshots[&id(1)].kills, 1);
    }
}
