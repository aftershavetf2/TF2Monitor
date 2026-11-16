use diesel::{
    connection::SimpleConnection,
    prelude::*,
    sql_types::{BigInt, Text},
};
use std::path::Path;

use super::{
    Result,
    models::{Community, ExportBan},
};

#[derive(QueryableByName)]
struct Number {
    #[diesel(sql_type = BigInt)]
    value: i64,
}

#[derive(QueryableByName)]
struct TextValue {
    #[diesel(sql_type = Text)]
    value: String,
}

#[derive(Debug, QueryableByName)]
pub struct StoredBan {
    #[diesel(sql_type = Text)]
    pub source: String,
    #[diesel(sql_type = Text)]
    pub record: String,
}

#[derive(Default, Clone, Debug, QueryableByName)]
pub struct UpdateState {
    #[diesel(sql_type = diesel::sql_types::Nullable<BigInt>)]
    pub last_attempt: Option<i64>,
    #[diesel(sql_type = diesel::sql_types::Nullable<BigInt>)]
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "Update metadata retained for diagnostics and verified by storage tests"
        )
    )]
    pub last_success: Option<i64>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    pub release: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    pub last_error: Option<String>,
}

pub fn open(path: &Path) -> Result<SqliteConnection> {
    let mut conn = SqliteConnection::establish(path.to_str().ok_or("Invalid database path")?)?;
    conn.batch_execute(
        "PRAGMA busy_timeout = 15000; PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;",
    )?;
    conn.batch_execute(
        "CREATE TABLE IF NOT EXISTS communities (
            id TEXT PRIMARY KEY NOT NULL, name TEXT NOT NULL, sha256 TEXT NOT NULL,
            ban_count INTEGER NOT NULL, complete INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS bans (
            community_id TEXT NOT NULL REFERENCES communities(id) ON DELETE CASCADE,
            steam_id INTEGER NOT NULL, record TEXT NOT NULL
         );
         CREATE INDEX IF NOT EXISTS bans_steam_id ON bans(steam_id);
         CREATE TABLE IF NOT EXISTS expected_communities (
            id TEXT PRIMARY KEY NOT NULL, complete INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS update_state (
            id INTEGER PRIMARY KEY CHECK(id=1), last_attempt INTEGER,
            last_success INTEGER, release TEXT, last_error TEXT
         );
         INSERT OR IGNORE INTO update_state (id) VALUES (1);",
    )?;
    Ok(conn)
}

pub fn state(conn: &mut SqliteConnection) -> Result<UpdateState> {
    Ok(diesel::sql_query(
        "SELECT last_attempt, last_success, release, last_error FROM update_state WHERE id=1",
    )
    .get_result(conn)?)
}

pub fn record_attempt(conn: &mut SqliteConnection, now: i64) -> Result<()> {
    diesel::sql_query("UPDATE update_state SET last_attempt=?, last_error=NULL WHERE id=1")
        .bind::<BigInt, _>(now)
        .execute(conn)?;
    Ok(())
}

pub fn record_error(conn: &mut SqliteConnection, error: &str) -> Result<()> {
    diesel::sql_query("UPDATE update_state SET last_error=? WHERE id=1")
        .bind::<Text, _>(error)
        .execute(conn)?;
    Ok(())
}

pub fn record_success(conn: &mut SqliteConnection, now: i64, release: &str) -> Result<()> {
    diesel::sql_query(
        "UPDATE update_state SET last_success=?, release=?, last_error=NULL WHERE id=1",
    )
    .bind::<BigInt, _>(now)
    .bind::<Text, _>(release)
    .execute(conn)?;
    Ok(())
}

pub fn content_hash(conn: &mut SqliteConnection, id: &str) -> Result<Option<String>> {
    Ok(
        diesel::sql_query("SELECT sha256 AS value FROM communities WHERE id=?")
            .bind::<Text, _>(id)
            .get_result::<TextValue>(conn)
            .optional()?
            .map(|row| row.value),
    )
}

pub fn update_metadata(conn: &mut SqliteConnection, community: &Community) -> QueryResult<usize> {
    diesel::sql_query("UPDATE communities SET name=?, complete=? WHERE id=?")
        .bind::<Text, _>(&community.name)
        .bind::<diesel::sql_types::Bool, _>(community.complete)
        .bind::<Text, _>(&community.id)
        .execute(conn)
}

/// Replace a full community snapshot, including expired and removed bans.
/// Validate before deleting; any import error rolls back the entire community.
pub fn import(
    conn: &mut SqliteConnection,
    community: &Community,
    bans: &[ExportBan],
) -> Result<()> {
    if bans.len() != community.ban_count {
        return Err(format!("Ban count mismatch for {}", community.id).into());
    }
    for ban in bans {
        ban.validate()?;
    }
    conn.transaction::<_, Box<dyn std::error::Error + Send + Sync>, _>(|conn| {
        diesel::sql_query("INSERT INTO communities (id,name,sha256,ban_count,complete) VALUES (?,?,?,?,?)
            ON CONFLICT(id) DO UPDATE SET name=excluded.name,sha256=excluded.sha256,ban_count=excluded.ban_count,complete=excluded.complete")
            .bind::<Text,_>(&community.id).bind::<Text,_>(&community.name)
            .bind::<Text,_>(&community.sha256).bind::<BigInt,_>(bans.len() as i64)
            .bind::<diesel::sql_types::Bool,_>(community.complete).execute(conn)?;
        diesel::sql_query("DELETE FROM bans WHERE community_id=?").bind::<Text,_>(&community.id).execute(conn)?;
        for ban in bans {
            diesel::sql_query("INSERT INTO bans (community_id,steam_id,record) VALUES (?,?,?)")
                .bind::<Text,_>(&community.id).bind::<BigInt,_>(ban.steam_id()?)
                .bind::<Text,_>(serde_json::to_string(ban)?).execute(conn)?;
        }
        Ok(())
    })
}

/// Returns None until at least one community has been imported.
pub fn coverage(conn: &mut SqliteConnection) -> Result<Option<bool>> {
    let count = diesel::sql_query("SELECT count(*) AS value FROM communities")
        .get_result::<Number>(conn)?
        .value;
    if count == 0 {
        return Ok(None);
    }
    let incomplete =
        diesel::sql_query("SELECT count(*) AS value FROM communities WHERE complete=0")
            .get_result::<Number>(conn)?
            .value;
    let expected = diesel::sql_query("SELECT count(*) AS value FROM expected_communities")
        .get_result::<Number>(conn)?
        .value;
    let missing_or_incomplete = diesel::sql_query(
        "SELECT count(*) AS value FROM expected_communities e
         LEFT JOIN communities c ON c.id=e.id WHERE c.id IS NULL OR e.complete=0",
    )
    .get_result::<Number>(conn)?
    .value;
    Ok(Some(
        incomplete == 0 && expected > 0 && missing_or_incomplete == 0,
    ))
}

pub fn lookup(conn: &mut SqliteConnection, steam_id: i64) -> Result<Vec<StoredBan>> {
    Ok(diesel::sql_query("SELECT communities.name AS source, bans.record FROM bans
        JOIN communities ON communities.id=bans.community_id WHERE steam_id=? ORDER BY communities.name")
        .bind::<BigInt,_>(steam_id).load(conn)?)
}

pub fn retain_communities(conn: &mut SqliteConnection, communities: &[Community]) -> Result<()> {
    let existing =
        diesel::sql_query("SELECT id AS value FROM communities").load::<TextValue>(conn)?;
    conn.transaction::<_, diesel::result::Error, _>(|conn| {
        for row in existing {
            if !communities
                .iter()
                .any(|community| community.id == row.value)
            {
                diesel::sql_query("DELETE FROM communities WHERE id=?")
                    .bind::<Text, _>(row.value)
                    .execute(conn)?;
            }
        }
        Ok(())
    })?;
    Ok(())
}

/// Record coverage before importing so a partial first download cannot appear complete.
pub fn expect_communities(conn: &mut SqliteConnection, communities: &[Community]) -> Result<()> {
    conn.transaction::<_, diesel::result::Error, _>(|conn| {
        diesel::sql_query("DELETE FROM expected_communities").execute(conn)?;
        for community in communities {
            diesel::sql_query("INSERT INTO expected_communities (id,complete) VALUES (?,?)")
                .bind::<Text, _>(&community.id)
                .bind::<diesel::sql_types::Bool, _>(community.complete)
                .execute(conn)?;
        }
        Ok(())
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ban_data::{
        tests::{ban, community},
        updater::{CHECK_INTERVAL, due},
    };

    #[test]
    fn snapshots_replace_history_and_reject_bad_data_without_losing_previous_bans() {
        let mut conn = open(Path::new(":memory:")).unwrap();
        assert_eq!(coverage(&mut conn).unwrap(), None);
        let mut other = community();
        other.id = "other".into();
        expect_communities(&mut conn, &[community(), other]).unwrap();
        let mut community = community();
        let record = ban();
        import(&mut conn, &community, std::slice::from_ref(&record)).unwrap();
        assert_eq!(
            lookup(&mut conn, record.steam_id().unwrap()).unwrap().len(),
            1
        );
        assert_eq!(coverage(&mut conn).unwrap(), Some(false));
        expect_communities(&mut conn, &[crate::ban_data::tests::community()]).unwrap();
        assert_eq!(coverage(&mut conn).unwrap(), Some(true));
        community.complete = false;
        update_metadata(&mut conn, &community).unwrap();
        assert_eq!(coverage(&mut conn).unwrap(), Some(false));
        let mut invalid = record.clone();
        invalid.steam_id = "broken".into();
        assert!(import(&mut conn, &community, &[invalid]).is_err());
        assert!(import(&mut conn, &community, &[]).is_err());
        assert_eq!(
            lookup(&mut conn, record.steam_id().unwrap()).unwrap().len(),
            1
        );
        // An insertion error after DELETE must roll back both data and the new hash.
        conn.batch_execute("CREATE TRIGGER fail_import BEFORE INSERT ON bans BEGIN SELECT RAISE(ABORT, 'test failure'); END;").unwrap();
        let original_hash = community.sha256.clone();
        community.sha256 = "c".repeat(64);
        assert!(import(&mut conn, &community, std::slice::from_ref(&record)).is_err());
        assert_eq!(
            content_hash(&mut conn, "example").unwrap().as_deref(),
            Some(original_hash.as_str())
        );
        assert_eq!(
            lookup(&mut conn, record.steam_id().unwrap()).unwrap().len(),
            1
        );
        conn.batch_execute("DROP TRIGGER fail_import;").unwrap();
        let mut removed = record.clone();
        removed.removed_at = Some("2026-11-01".into());
        import(&mut conn, &community, &[removed]).unwrap();
        let found = lookup(&mut conn, record.steam_id().unwrap()).unwrap();
        assert_eq!(found.len(), 1);
        assert!(found[0].record.contains("2026-11-01"));
        retain_communities(&mut conn, &[]).unwrap();
        assert!(
            lookup(&mut conn, record.steam_id().unwrap())
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn failed_and_manual_attempts_survive_reopening_and_throttle_automatic_checks() {
        let path = std::env::temp_dir().join(format!(
            "tf2monitor-ban-state-{}.sqlite",
            std::process::id()
        ));
        let mut conn = open(&path).unwrap();
        let now = 1794730000;
        assert!(due(state(&mut conn).unwrap().last_attempt, now));
        record_attempt(&mut conn, now).unwrap();
        record_error(&mut conn, "Offline").unwrap();
        drop(conn);
        let mut conn = open(&path).unwrap();
        let saved = state(&mut conn).unwrap();
        assert_eq!(saved.last_error.as_deref(), Some("Offline"));
        assert_eq!(saved.last_success, None);
        assert!(!due(saved.last_attempt, now + CHECK_INTERVAL - 1));
        assert!(due(saved.last_attempt, now + CHECK_INTERVAL));
        record_attempt(&mut conn, now + 20).unwrap(); // Manual check resets the interval.
        record_success(&mut conn, now + 21, "release-1").unwrap();
        assert!(!due(
            state(&mut conn).unwrap().last_attempt,
            now + CHECK_INTERVAL
        ));
        assert_eq!(
            state(&mut conn).unwrap().release.as_deref(),
            Some("release-1")
        );
        assert!(state(&mut conn).unwrap().last_error.is_none());
        drop(conn);
        std::fs::remove_file(path).unwrap();
    }
}
