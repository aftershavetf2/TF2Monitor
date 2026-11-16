//! Database-backed markings with a one-time legacy import and a JSON export.
use std::{
    collections::HashMap,
    error::Error,
    path::Path,
    time::{Duration, Instant},
};

use chrono::Utc;
use diesel::{prelude::*, sql_types::Text};

use super::models::{PlayerAttribute, PlayerInfo, PlayerLastSeen, TF2BDPlayerList};
use crate::{
    db::{
        connection::DbPool,
        entities::{NewAccount, NewPlayerFlag},
        queries,
        schema::{account, player_flags},
    },
    models::steamid::SteamID,
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub struct RulesetHandler {
    source: String,
    db: DbPool,
    template: TF2BDPlayerList,
    player_rules: HashMap<SteamID, PlayerInfo>,
    export_pending: bool,
    last_export_attempt: Option<Instant>,
}

impl RulesetHandler {
    pub fn new(source: &str, db: &DbPool) -> Result<Self> {
        let mut conn = db.get()?;
        let template = initialize(&mut conn, Path::new(source))?;
        let player_rules = load_markings(&mut conn)?;
        drop(conn);
        let mut handler = Self {
            source: source.into(),
            db: db.clone(),
            template,
            player_rules,
            export_pending: true,
            last_export_attempt: None,
        };
        handler.sync_export();
        Ok(handler)
    }

    pub fn get_player_marking(&self, steamid: &SteamID) -> Option<&PlayerInfo> {
        self.player_rules.get(steamid)
    }

    pub fn set_player_flags(
        &mut self,
        steamid: SteamID,
        player_name: Option<&str>,
        flag: PlayerAttribute,
        enable: bool,
    ) -> Result<()> {
        let mut conn = self.db.get()?;
        set_flag(&mut conn, steamid, player_name, flag, enable)?;
        self.export_pending = true;
        self.last_export_attempt = None;
        // Only publish committed database state, never an optimistic file-based update.
        self.player_rules = load_markings(&mut conn)?;
        drop(conn);
        self.sync_export();
        Ok(())
    }

    /// Failed exports are retried; the committed database remains authoritative.
    pub fn sync_export(&mut self) {
        if !self.export_pending
            || self
                .last_export_attempt
                .is_some_and(|time| time.elapsed() < Duration::from_secs(5))
        {
            return;
        }
        self.last_export_attempt = Some(Instant::now());
        let result = (|| -> Result<()> {
            let mut list = self.template.clone();
            // Read the export from the database too, including offline players.
            let mut conn = self.db.get()?;
            self.player_rules = load_markings(&mut conn)?;
            drop(conn);
            list.players = self.player_rules.values().cloned().collect();
            list.players
                .sort_by_key(|player| SteamID::from_steam_id32(&player.steamid32).to_u64());
            export(&list, Path::new(&self.source))
        })();
        match result {
            Ok(()) => self.export_pending = false,
            Err(error) => log::error!(
                "Could not export player markings to {} (will retry): {}",
                self.source,
                error
            ),
        }
    }
}

#[derive(QueryableByName)]
struct TemplateRow {
    #[diesel(sql_type = Text)]
    template: String,
}

/// The metadata row is also the migration marker, committed with the imported flags.
fn initialize(conn: &mut SqliteConnection, source: &Path) -> Result<TF2BDPlayerList> {
    conn.transaction::<_, Box<dyn Error>, _>(|conn| {
        diesel::sql_query("CREATE TABLE IF NOT EXISTS player_markings_metadata (id INTEGER PRIMARY KEY CHECK (id = 1), template TEXT NOT NULL)").execute(conn)?;
        if let Some(row) = diesel::sql_query("SELECT template FROM player_markings_metadata WHERE id = 1").get_result::<TemplateRow>(conn).optional()? {
            return Ok(serde_json::from_str(&row.template)?);
        }
        let mut list = if source.exists() {
            serde_json::from_slice::<TF2BDPlayerList>(&std::fs::read(source)?)?
        } else {
            TF2BDPlayerList::new()
        };
        for player in &list.players {
            let steamid = SteamID::from_steam_id32(&player.steamid32);
            ensure_account(conn, steamid, player.last_seen.as_ref().and_then(|last| last.player_name.as_deref()))?;
            for flag in &player.attributes {
                let time = player.last_seen.as_ref().map_or(0, |last| last.time);
                diesel::insert_into(player_flags::table).values(NewPlayerFlag {
                    steam_id: steamid.to_u64() as i64, flag_type: format!("{flag:?}"),
                    source: "playerlist.json".into(), first_seen: time, last_seen: time, notified: false,
                }).on_conflict_do_nothing().execute(conn)?;
            }
        }
        for flag in queries::get_all_player_flags(conn)? {
            let steamid = SteamID::from_u64(flag.steam_id as u64);
            if steamid.is_valid() { ensure_account(conn, steamid, None)?; }
        }
        // Keep file metadata and rules in SQLite, without maintaining a second player list.
        list.players.clear();
        diesel::sql_query("INSERT INTO player_markings_metadata (id, template) VALUES (1, ?)")
            .bind::<Text, _>(serde_json::to_string(&list)?).execute(conn)?;
        Ok(list)
    })
}

fn ensure_account(
    conn: &mut SqliteConnection,
    steamid: SteamID,
    name: Option<&str>,
) -> QueryResult<()> {
    diesel::insert_into(account::table)
        .values(NewAccount {
            steam_id: steamid.to_u64() as i64,
            name: name.unwrap_or("Unknown").into(),
            created_date: None,
            avatar_thumb_url: String::new(),
            avatar_full_url: String::new(),
            public_profile: false,
            last_updated: 0,
            friends_fetched: None,
            comments_fetched: None,
            playtimes_fetched: None,
            steam_bans_last_fetched: None,
        })
        .on_conflict(account::steam_id)
        .do_nothing()
        .execute(conn)?;
    Ok(())
}

fn set_flag(
    conn: &mut SqliteConnection,
    steamid: SteamID,
    name: Option<&str>,
    flag: PlayerAttribute,
    enable: bool,
) -> QueryResult<()> {
    conn.transaction(|conn| {
        ensure_account(conn, steamid, name)?;
        if enable {
            let time = Utc::now().timestamp();
            queries::upsert_player_flag(
                conn,
                NewPlayerFlag {
                    steam_id: steamid.to_u64() as i64,
                    flag_type: format!("{flag:?}"),
                    source: "manual".into(),
                    first_seen: time,
                    last_seen: time,
                    notified: false,
                },
            )?;
        } else {
            // All sources represent the same editable marking. Remove legacy copies too.
            queries::remove_player_flag(conn, steamid.to_u64() as i64, &format!("{flag:?}"))?;
        }
        Ok(())
    })
}

fn load_markings(conn: &mut SqliteConnection) -> Result<HashMap<SteamID, PlayerInfo>> {
    conn.transaction::<_, Box<dyn Error>, _>(|conn| {
        let accounts: HashMap<_, _> = queries::get_all_accounts(conn)?
            .into_iter()
            .map(|a| (a.steam_id, a))
            .collect();
        let mut players: HashMap<SteamID, PlayerInfo> = HashMap::new();
        for flag in queries::get_all_player_flags(conn)? {
            let attribute = match flag.flag_type.as_str() {
                "Cheater" => PlayerAttribute::Cheater,
                "Bot" => PlayerAttribute::Bot,
                "Suspicious" => PlayerAttribute::Suspicious,
                "Toxic" | "Racist" => PlayerAttribute::Toxic,
                "Exploiter" => PlayerAttribute::Exploiter,
                "Cool" => PlayerAttribute::Cool,
                other => {
                    log::warn!("Unknown database player flag: {}", other);
                    continue;
                }
            };
            let steamid = SteamID::from_u64(flag.steam_id as u64);
            if !steamid.is_valid() {
                continue;
            }
            let player = players.entry(steamid).or_insert_with(|| PlayerInfo {
                steamid32: steamid.to_steam_id32(),
                attributes: Vec::new(),
                last_seen: Some(PlayerLastSeen {
                    player_name: accounts.get(&flag.steam_id).map(|a| a.name.clone()),
                    time: flag.last_seen,
                }),
            });
            if !player.attributes.contains(&attribute) {
                player.attributes.push(attribute);
            }
            if let Some(last) = &mut player.last_seen {
                last.time = last.time.max(flag.last_seen);
            }
        }
        Ok(players)
    })
}

fn export(list: &TF2BDPlayerList, source: &Path) -> Result<()> {
    use std::io::Write;
    let temporary = source.with_extension("json.tmp");
    let mut file = std::fs::File::create(&temporary)?;
    file.write_all(&serde_json::to_vec_pretty(list)?)?;
    file.sync_all()?;
    drop(file);
    std::fs::rename(temporary, source)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::r2d2::{ConnectionManager, Pool};
    use std::{
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
    };

    struct Fixture {
        directory: PathBuf,
        db: DbPool,
    }
    impl Fixture {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let directory = std::env::temp_dir().join(format!(
                "tf2monitor-markings-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&directory).unwrap();
            let db = Pool::builder()
                .max_size(1)
                .build(ConnectionManager::<SqliteConnection>::new(":memory:"))
                .unwrap();
            crate::db::connection::setup_schema(&mut db.get().unwrap()).unwrap();
            Self { directory, db }
        }
        fn path(&self) -> PathBuf {
            self.directory.join("playerlist.json")
        }
        fn handler(&self) -> RulesetHandler {
            RulesetHandler::new(self.path().to_str().unwrap(), &self.db).unwrap()
        }
        fn read(&self) -> TF2BDPlayerList {
            serde_json::from_slice(&std::fs::read(self.path()).unwrap()).unwrap()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.directory);
        }
    }
    fn id(number: u64) -> SteamID {
        SteamID::from_u64(crate::models::steamid::MIN_STEAMID64 + number)
    }
    fn seed(conn: &mut SqliteConnection, steamid: SteamID, flag: PlayerAttribute, source: &str) {
        ensure_account(conn, steamid, Some("aftershave")).unwrap();
        queries::upsert_player_flag(
            conn,
            NewPlayerFlag {
                steam_id: steamid.to_u64() as i64,
                flag_type: format!("{flag:?}"),
                source: source.into(),
                first_seen: 100,
                last_seen: 200,
                notified: false,
            },
        )
        .unwrap();
    }

    #[test]
    fn imports_once_merges_database_only_players_and_keeps_metadata() {
        let fixture = Fixture::new();
        seed(
            &mut fixture.db.get().unwrap(),
            id(1),
            PlayerAttribute::Cool,
            "manual",
        );
        seed(
            &mut fixture.db.get().unwrap(),
            id(2),
            PlayerAttribute::Bot,
            "playerlist.json",
        );
        let legacy = serde_json::json!({
            "$schema": TF2BDPlayerList::new().schema,
            "file_info": {"authors": ["test"], "description": "test", "title": "Test", "update_url": ""},
            "rules": [],
            "players": [{"steamid": id(1).to_steam_id32(), "attributes": ["cheater"], "last_seen": {"player_name": "aftershave", "time": 123}}]
        });
        std::fs::write(fixture.path(), serde_json::to_vec(&legacy).unwrap()).unwrap();
        let mut handler = fixture.handler();
        let marking = handler.get_player_marking(&id(1)).unwrap();
        assert_eq!(
            marking.attributes,
            vec![PlayerAttribute::Cheater, PlayerAttribute::Cool]
        );
        assert!(handler.get_player_marking(&id(2)).is_some());
        assert_eq!(fixture.read().players.len(), 2);
        assert_eq!(
            serde_json::to_value(fixture.read()).unwrap()["file_info"],
            legacy["file_info"]
        );
        handler
            .set_player_flags(id(1), Some("aftershave"), PlayerAttribute::Cheater, false)
            .unwrap();
        // An old or externally edited export must never re-import a removed flag.
        std::fs::write(fixture.path(), serde_json::to_vec(&legacy).unwrap()).unwrap();
        let restarted = fixture.handler();
        assert_eq!(
            restarted.get_player_marking(&id(1)).unwrap().attributes,
            vec![PlayerAttribute::Cool]
        );
        assert_eq!(fixture.read().players.len(), 2);
    }

    #[test]
    fn edits_remove_all_source_copies_and_survive_restart_without_json() {
        let fixture = Fixture::new();
        for source in ["manual", "playerlist.json"] {
            seed(
                &mut fixture.db.get().unwrap(),
                id(1),
                PlayerAttribute::Cheater,
                source,
            );
        }
        let mut handler = fixture.handler();
        assert_eq!(
            handler.get_player_marking(&id(1)).unwrap().attributes.len(),
            1
        );
        handler
            .set_player_flags(id(1), Some("aftershave"), PlayerAttribute::Cheater, false)
            .unwrap();
        assert!(
            queries::get_all_player_flags(&mut fixture.db.get().unwrap())
                .unwrap()
                .is_empty()
        );
        assert!(handler.get_player_marking(&id(1)).is_none());
        assert!(fixture.read().players.is_empty());
        handler
            .set_player_flags(id(1), Some("aftershave"), PlayerAttribute::Cool, true)
            .unwrap();
        std::fs::remove_file(fixture.path()).unwrap();
        let restarted = fixture.handler();
        assert_eq!(
            restarted.get_player_marking(&id(1)).unwrap().attributes,
            vec![PlayerAttribute::Cool]
        );
        assert_eq!(
            fixture.read().players[0].attributes,
            vec![PlayerAttribute::Cool]
        );
        // After migration even a broken JSON file is repaired from the database.
        std::fs::write(fixture.path(), "broken json").unwrap();
        fixture.handler();
        assert_eq!(
            fixture.read().players[0].attributes,
            vec![PlayerAttribute::Cool]
        );
    }

    #[test]
    fn failed_export_preserves_database_updates_and_retries() {
        let fixture = Fixture::new();
        let mut handler = fixture.handler();
        std::fs::remove_file(fixture.path()).unwrap();
        std::fs::create_dir(fixture.path()).unwrap();
        handler
            .set_player_flags(id(1), Some("aftershave"), PlayerAttribute::Cool, true)
            .unwrap();
        assert!(handler.export_pending);
        assert!(handler.get_player_marking(&id(1)).is_some());
        assert_eq!(
            queries::get_all_player_flags(&mut fixture.db.get().unwrap())
                .unwrap()
                .len(),
            1
        );
        std::fs::remove_dir(fixture.path()).unwrap();
        handler.last_export_attempt = None;
        handler.sync_export();
        assert!(!handler.export_pending);
        assert_eq!(
            fixture.read().players[0].attributes,
            vec![PlayerAttribute::Cool]
        );
    }

    #[test]
    fn failed_database_write_does_not_change_live_markings_or_export() {
        let fixture = Fixture::new();
        seed(
            &mut fixture.db.get().unwrap(),
            id(1),
            PlayerAttribute::Cheater,
            "playerlist.json",
        );
        let mut handler = fixture.handler();
        let original_export = std::fs::read(fixture.path()).unwrap();
        diesel::sql_query("CREATE TRIGGER reject_flag_delete BEFORE DELETE ON player_flags BEGIN SELECT RAISE(ABORT, 'test database failure'); END")
            .execute(&mut fixture.db.get().unwrap()).unwrap();
        assert!(
            handler
                .set_player_flags(id(1), Some("aftershave"), PlayerAttribute::Cheater, false)
                .is_err()
        );
        assert_eq!(
            handler.get_player_marking(&id(1)).unwrap().attributes,
            vec![PlayerAttribute::Cheater]
        );
        assert_eq!(std::fs::read(fixture.path()).unwrap(), original_export);
        assert_eq!(
            queries::get_all_player_flags(&mut fixture.db.get().unwrap())
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn failed_migration_can_be_retried_without_losing_database_flags() {
        let fixture = Fixture::new();
        seed(
            &mut fixture.db.get().unwrap(),
            id(1),
            PlayerAttribute::Cool,
            "manual",
        );
        std::fs::write(fixture.path(), "broken json").unwrap();
        assert!(RulesetHandler::new(fixture.path().to_str().unwrap(), &fixture.db).is_err());
        assert_eq!(
            queries::get_all_player_flags(&mut fixture.db.get().unwrap())
                .unwrap()
                .len(),
            1
        );
        std::fs::remove_file(fixture.path()).unwrap();
        assert!(fixture.handler().get_player_marking(&id(1)).is_some());
    }
}
