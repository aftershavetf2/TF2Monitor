use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager, CustomizeConnection, Pool};
use std::path::Path;

const DATABASE_FILE: &str = "appdata.sqlite";

pub type DbPool = Pool<ConnectionManager<SqliteConnection>>;

/// Connection customizer to configure SQLite for concurrent access
#[derive(Debug, Clone, Copy)]
struct SqliteConnectionCustomizer;

impl CustomizeConnection<SqliteConnection, diesel::r2d2::Error> for SqliteConnectionCustomizer {
    fn on_acquire(&self, conn: &mut SqliteConnection) -> Result<(), diesel::r2d2::Error> {
        use diesel::sql_query;

        // Enable WAL (Write-Ahead Logging) mode for better concurrent access
        // This allows multiple readers and one writer simultaneously
        sql_query("PRAGMA journal_mode = WAL")
            .execute(conn)
            .map_err(diesel::r2d2::Error::QueryError)?;

        // Set busy timeout to 15 seconds (15000ms)
        // SQLite will retry for this duration instead of immediately failing with "database is locked"
        sql_query("PRAGMA busy_timeout = 15000")
            .execute(conn)
            .map_err(diesel::r2d2::Error::QueryError)?;

        // Use NORMAL synchronous mode for better performance
        // Still safe with WAL mode, provides good balance of safety and speed
        sql_query("PRAGMA synchronous = NORMAL")
            .execute(conn)
            .map_err(diesel::r2d2::Error::QueryError)?;

        // Store temp tables in memory for better performance
        sql_query("PRAGMA temp_store = MEMORY")
            .execute(conn)
            .map_err(diesel::r2d2::Error::QueryError)?;

        // Enable foreign keys (best practice)
        sql_query("PRAGMA foreign_keys = ON")
            .execute(conn)
            .map_err(diesel::r2d2::Error::QueryError)?;

        Ok(())
    }
}

/// Connects to the SQLite database and sets up the schema if needed.
///
/// This function:
/// - Connects to the SQLite database file `appdata.sqlite`
/// - Creates the database file if it doesn't exist
/// - Creates all necessary tables based on the entity definitions
/// - Creates indexes as specified in the data model
///
/// # Returns
///
/// Returns a connection pool if successful, or an error if connection/setup fails.
pub fn connect() -> Result<DbPool, Box<dyn std::error::Error>> {
    let db_path = Path::new(DATABASE_FILE);

    // Check if database file exists, log accordingly
    if db_path.exists() {
        log::info!("Database file '{}' exists, connecting...", DATABASE_FILE);
    } else {
        log::info!(
            "Database file '{}' does not exist, will be created...",
            DATABASE_FILE
        );
    }

    // Migrate with a single connection before starting the pool or workers.
    {
        let mut conn = SqliteConnection::establish(DATABASE_FILE)?;
        diesel::sql_query("PRAGMA busy_timeout = 15000").execute(&mut conn)?;
        remove_legacy_sourcebans(&mut conn)?;
        setup_schema(&mut conn)?;
    }

    // Connect to SQLite database
    // SQLite will create the file if it doesn't exist
    let database_url = DATABASE_FILE;
    let manager = ConnectionManager::<SqliteConnection>::new(database_url);

    // Build connection pool with optimized settings for concurrent access
    let pool = r2d2::Pool::builder()
        .max_size(10) // Increased from 5 to handle multiple concurrent threads
        .min_idle(Some(2)) // Keep minimum 2 idle connections ready
        .connection_timeout(std::time::Duration::from_secs(30)) // Wait up to 30s for a connection
        .connection_customizer(Box::new(SqliteConnectionCustomizer)) // Apply WAL mode and other settings
        .build(manager)?;

    log::info!(
        "Connected to database '{}' with WAL mode enabled",
        DATABASE_FILE
    );

    Ok(pool)
}

/// Remove the obsolete cache and reclaim its pages once, outside a transaction.
/// A pending marker survives interruption between committing cleanup and VACUUM.
fn remove_legacy_sourcebans(conn: &mut SqliteConnection) -> QueryResult<()> {
    #[derive(QueryableByName)]
    struct Value {
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        value: i64,
    }
    diesel::sql_query(
        "CREATE TABLE IF NOT EXISTS app_migrations (
            name TEXT PRIMARY KEY NOT NULL,
            completed INTEGER NOT NULL
        )",
    )
    .execute(conn)?;
    let completed = diesel::sql_query(
        "SELECT completed AS value FROM app_migrations WHERE name = 'remove_legacy_sourcebans'",
    )
    .get_result::<Value>(conn)
    .optional()?;
    if completed.as_ref().is_some_and(|row| row.value == 1) {
        return Ok(());
    }
    let legacy = diesel::sql_query(
        "SELECT (EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table'
            AND name IN ('bans', 'ban_sources')) OR
            EXISTS(SELECT 1 FROM pragma_table_info('account') WHERE name = 'reputation_fetched')) AS value",
    ).get_result::<Value>(conn)?.value != 0;
    conn.transaction::<_, diesel::result::Error, _>(|conn| {
        diesel::sql_query("DROP TABLE IF EXISTS bans").execute(conn)?;
        diesel::sql_query("DROP TABLE IF EXISTS ban_sources").execute(conn)?;
        let column = diesel::sql_query(
            "SELECT count(*) AS value FROM pragma_table_info('account') WHERE name = 'reputation_fetched'",
        ).get_result::<Value>(conn)?;
        if column.value != 0 {
            diesel::sql_query("ALTER TABLE account DROP COLUMN reputation_fetched").execute(conn)?;
        }
        diesel::sql_query(
            "INSERT OR IGNORE INTO app_migrations (name, completed) VALUES ('remove_legacy_sourcebans', 0)",
        ).execute(conn)?;
        Ok(())
    })?;
    if legacy || completed.is_some() {
        log::info!("Compacting database after legacy SourceBans cleanup...");
        diesel::sql_query("VACUUM").execute(conn)?;
        // Return the WAL's reclaimed pages to disk as well.
        diesel::sql_query("PRAGMA wal_checkpoint(TRUNCATE)").execute(conn)?;
    }
    diesel::sql_query(
        "UPDATE app_migrations SET completed = 1 WHERE name = 'remove_legacy_sourcebans'",
    )
    .execute(conn)?;
    Ok(())
}

/// Sets up the database schema by creating all necessary tables and indexes.
///
/// This function uses raw SQL to create tables based on entity definitions.
/// It creates tables if they don't exist, ensuring the database schema matches
/// the current entity definitions.
fn setup_schema(conn: &mut SqliteConnection) -> Result<(), Box<dyn std::error::Error>> {
    // Create account table
    diesel::sql_query(
        "CREATE TABLE IF NOT EXISTS account (
            steam_id INTEGER PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            created_date INTEGER,
            avatar_thumb_url TEXT NOT NULL,
            avatar_full_url TEXT NOT NULL,
            public_profile INTEGER NOT NULL,
            last_updated INTEGER NOT NULL,
            friends_fetched INTEGER,
            comments_fetched INTEGER,
            playtimes_fetched INTEGER
        )",
    )
    .execute(conn)?;

    // Add steam_bans_last_fetched column if it doesn't exist (for existing databases)
    diesel::sql_query("ALTER TABLE account ADD COLUMN steam_bans_last_fetched INTEGER")
        .execute(conn)
        .ok(); // Ignore error if column already exists

    // Create friendship table
    diesel::sql_query(
        "CREATE TABLE IF NOT EXISTS friendship (
            steam_id INTEGER NOT NULL,
            friend_steam_id INTEGER NOT NULL,
            friend_name TEXT NOT NULL,
            friend_date INTEGER NOT NULL,
            unfriend_date INTEGER,
            PRIMARY KEY (steam_id, friend_steam_id)
        )",
    )
    .execute(conn)?;

    // Create comments table
    diesel::sql_query(
        "CREATE TABLE IF NOT EXISTS comments (
            id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
            steam_id INTEGER NOT NULL,
            writer_steam_id INTEGER NOT NULL,
            writer_name TEXT NOT NULL,
            comment TEXT NOT NULL,
            created_date INTEGER NOT NULL,
            deleted_date INTEGER
        )",
    )
    .execute(conn)?;

    migrate_comment_avatars(conn)?;

    // Create playtime table
    diesel::sql_query(
        "CREATE TABLE IF NOT EXISTS playtime (
            steam_id INTEGER NOT NULL,
            game TEXT NOT NULL,
            play_minutes INTEGER,
            last_updated INTEGER NOT NULL,
            PRIMARY KEY (steam_id, game)
        )",
    )
    .execute(conn)?;

    // Migrate existing playtime table to make play_minutes nullable
    // SQLite doesn't support ALTER COLUMN, so we need to recreate the table if needed
    // Check table schema using PRAGMA table_info and migrate if play_minutes is NOT NULL

    let migration_result: Result<(), Box<dyn std::error::Error>> = (|| {
        // Use PRAGMA to check if play_minutes column allows NULL
        // In the result, 'notnull' will be 1 if NOT NULL, 0 if NULL allowed
        #[derive(diesel::QueryableByName)]
        struct ColumnInfo {
            #[diesel(sql_type = diesel::sql_types::Integer)]
            #[diesel(column_name = "notnull")]
            notnull: i32,
        }

        let column_info: Result<ColumnInfo, _> = diesel::sql_query(
            "SELECT * FROM pragma_table_info('playtime') WHERE name = 'play_minutes'",
        )
        .get_result(conn);

        // Check if column is NOT NULL (notnull=1)
        let needs_migration = match &column_info {
            Ok(info) => {
                log::info!("PRAGMA check: play_minutes notnull={}", info.notnull);
                info.notnull == 1
            }
            Err(e) => {
                log::warn!("PRAGMA check failed: {}, assuming migration needed", e);
                true // If we can't check, assume migration is needed
            }
        };

        if !needs_migration {
            log::info!("Playtime table already supports NULL play_minutes, no migration needed");
            return Ok(());
        }

        // Migration needed - recreate the table
        log::info!("Migrating playtime table to support NULL play_minutes...");

        // Create new table with correct schema
        diesel::sql_query(
            "CREATE TABLE playtime_new (
                steam_id INTEGER NOT NULL,
                game TEXT NOT NULL,
                play_minutes INTEGER,
                last_updated INTEGER NOT NULL,
                PRIMARY KEY (steam_id, game)
            )",
        )
        .execute(conn)?;

        // Copy data, converting 0 to NULL for unknown playtime
        diesel::sql_query(
            "INSERT INTO playtime_new (steam_id, game, play_minutes, last_updated)
             SELECT steam_id, game, CASE WHEN play_minutes = 0 THEN NULL ELSE play_minutes END, last_updated
             FROM playtime"
        ).execute(conn)?;

        // Swap tables
        diesel::sql_query("DROP TABLE playtime").execute(conn)?;
        diesel::sql_query("ALTER TABLE playtime_new RENAME TO playtime").execute(conn)?;

        log::info!("Successfully migrated playtime table");
        Ok(())
    })();

    if let Err(e) = migration_result {
        log::error!("Playtime table migration failed: {}", e);
    }

    // Create player_flags table
    diesel::sql_query(
        "CREATE TABLE IF NOT EXISTS player_flags (
            steam_id INTEGER NOT NULL,
            flag_type TEXT NOT NULL,
            source TEXT NOT NULL,
            first_seen INTEGER NOT NULL,
            last_seen INTEGER NOT NULL,
            notified INTEGER NOT NULL,
            PRIMARY KEY (steam_id, flag_type, source)
        )",
    )
    .execute(conn)?;

    // Create steam_bans table
    // Note: steam_id is NOT a foreign key - we want to store bans for players
    // that may not yet exist in the account table
    diesel::sql_query(
        "CREATE TABLE IF NOT EXISTS steam_bans (
            steam_id INTEGER PRIMARY KEY NOT NULL,
            community_banned INTEGER NOT NULL,
            vac_banned INTEGER NOT NULL,
            number_of_vac_bans INTEGER NOT NULL,
            days_since_last_ban INTEGER NOT NULL,
            number_of_game_bans INTEGER NOT NULL,
            economy_ban TEXT NOT NULL
        )",
    )
    .execute(conn)?;

    // Create indexes as specified in DATAMODEL.md
    // Note: Primary keys are automatically indexed, so we only need to create additional indexes

    diesel::sql_query("CREATE INDEX IF NOT EXISTS idx_friendship_steam_id ON friendship(steam_id)")
        .execute(conn)?;
    diesel::sql_query(
        "CREATE INDEX IF NOT EXISTS idx_friendship_friend_steam_id ON friendship(friend_steam_id)",
    )
    .execute(conn)?;
    diesel::sql_query("CREATE INDEX IF NOT EXISTS idx_comments_steam_id ON comments(steam_id)")
        .execute(conn)?;
    diesel::sql_query(
        "CREATE INDEX IF NOT EXISTS idx_comments_writer_steam_id ON comments(writer_steam_id)",
    )
    .execute(conn)?;
    diesel::sql_query(
        "CREATE INDEX IF NOT EXISTS idx_comments_created_date ON comments(created_date)",
    )
    .execute(conn)?;
    diesel::sql_query("CREATE INDEX IF NOT EXISTS idx_playtime_steam_id ON playtime(steam_id)")
        .execute(conn)?;
    diesel::sql_query("CREATE INDEX IF NOT EXISTS idx_playtime_game ON playtime(game)")
        .execute(conn)?;
    diesel::sql_query(
        "CREATE INDEX IF NOT EXISTS idx_player_flags_steam_id ON player_flags(steam_id)",
    )
    .execute(conn)?;
    diesel::sql_query(
        "CREATE INDEX IF NOT EXISTS idx_player_flags_notified ON player_flags(notified)",
    )
    .execute(conn)?;
    diesel::sql_query("CREATE INDEX IF NOT EXISTS idx_steam_bans_steam_id ON steam_bans(steam_id)")
        .execute(conn)?;

    log::info!("Database schema setup completed");
    Ok(())
}

/// Add thumbnail storage without discarding comments from existing installations.
fn migrate_comment_avatars(conn: &mut SqliteConnection) -> QueryResult<()> {
    #[derive(QueryableByName)]
    struct Column {
        #[diesel(sql_type = diesel::sql_types::Text)]
        name: String,
    }
    let columns = diesel::sql_query("PRAGMA table_info(comments)").load::<Column>(conn)?;
    if !columns
        .iter()
        .any(|column| column.name == "writer_avatar_thumb")
    {
        diesel::sql_query(
            "ALTER TABLE comments ADD COLUMN writer_avatar_thumb TEXT NOT NULL DEFAULT ''",
        )
        .execute(conn)?;
    }
    Ok(())
}

#[cfg(test)]
mod comment_avatar_tests {
    use super::*;
    use crate::db::{entities::NewComment, queries};

    #[test]
    fn avatar_migration_preserves_comments_and_backfills_cached_authors() {
        let mut conn = SqliteConnection::establish(":memory:").unwrap();
        diesel::sql_query(
            "CREATE TABLE comments (
            id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
            steam_id INTEGER NOT NULL, writer_steam_id INTEGER NOT NULL,
            writer_name TEXT NOT NULL, comment TEXT NOT NULL,
            created_date INTEGER NOT NULL, deleted_date INTEGER
        )",
        )
        .execute(&mut conn)
        .unwrap();
        diesel::sql_query(
            "INSERT INTO comments (steam_id, writer_steam_id, writer_name, comment, created_date)
            VALUES (1, 2, 'Author', 'Cached comment', 100)",
        )
        .execute(&mut conn)
        .unwrap();
        migrate_comment_avatars(&mut conn).unwrap();
        migrate_comment_avatars(&mut conn).unwrap();
        let cached = queries::get_active_comments_for_account(&mut conn, 1).unwrap();
        assert_eq!(cached.len(), 1);
        assert_eq!(cached[0].comment, "Cached comment");
        assert!(cached[0].writer_avatar_thumb.is_empty());

        queries::insert_comment(
            &mut conn,
            NewComment {
                steam_id: 3,
                writer_steam_id: 2,
                writer_name: "Author".into(),
                writer_avatar_thumb: "https://example.com/avatar.jpg".into(),
                comment: "New comment".into(),
                created_date: 200,
                deleted_date: None,
            },
        )
        .unwrap();
        let cached = queries::get_active_comments_for_account(&mut conn, 1).unwrap();
        assert_eq!(cached.len(), 1);
        assert_eq!(
            cached[0].writer_avatar_thumb,
            "https://example.com/avatar.jpg"
        );
        let fresh = queries::get_active_comments_for_account(&mut conn, 3).unwrap();
        assert_eq!(fresh[0].writer_avatar_thumb, cached[0].writer_avatar_thumb);
    }
}

#[cfg(test)]
mod legacy_sourcebans_tests {
    use super::*;
    use diesel::connection::SimpleConnection;

    #[derive(Debug, PartialEq, QueryableByName)]
    struct Value {
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        value: i64,
    }

    fn value(conn: &mut SqliteConnection, sql: &str) -> i64 {
        diesel::sql_query(sql)
            .get_result::<Value>(conn)
            .unwrap()
            .value
    }

    #[test]
    fn cleanup_preserves_other_data_compacts_and_only_runs_once() {
        let path =
            std::env::temp_dir().join(format!("tf2monitor-cleanup-{}.sqlite", std::process::id()));
        let mut conn = SqliteConnection::establish(path.to_str().unwrap()).unwrap();
        conn.batch_execute(
            "PRAGMA journal_mode = WAL;
             CREATE TABLE account (steam_id INTEGER PRIMARY KEY, name TEXT, reputation_fetched INTEGER);
             INSERT INTO account VALUES (1, 'Player', 100);
             CREATE TABLE bans (payload BLOB);
             INSERT INTO bans VALUES (zeroblob(1048576));
             CREATE TABLE ban_sources (name TEXT);
             INSERT INTO ban_sources VALUES ('legacy');
             CREATE TABLE steam_bans (steam_id INTEGER PRIMARY KEY);
             INSERT INTO steam_bans VALUES (1);
             CREATE TABLE player_flags (steam_id INTEGER PRIMARY KEY);
             INSERT INTO player_flags VALUES (1);"
        ).unwrap();
        let before = value(
            &mut conn,
            "SELECT page_count AS value FROM pragma_page_count",
        );
        remove_legacy_sourcebans(&mut conn).unwrap();
        let after = value(
            &mut conn,
            "SELECT page_count AS value FROM pragma_page_count",
        );
        assert!(after < before);
        assert_eq!(
            value(
                &mut conn,
                "SELECT count(*) AS value FROM sqlite_master WHERE name IN ('bans', 'ban_sources')"
            ),
            0
        );
        assert_eq!(
            value(
                &mut conn,
                "SELECT count(*) AS value FROM pragma_table_info('account') WHERE name = 'reputation_fetched'"
            ),
            0
        );
        assert_eq!(
            value(
                &mut conn,
                "SELECT count(*) AS value FROM account WHERE steam_id = 1 AND name = 'Player'"
            ),
            1
        );
        assert_eq!(
            value(&mut conn, "SELECT count(*) AS value FROM steam_bans"),
            1
        );
        assert_eq!(
            value(&mut conn, "SELECT count(*) AS value FROM player_flags"),
            1
        );
        // Free pages added after migration must survive the second call (no VACUUM).
        conn.batch_execute("CREATE TABLE scratch (payload BLOB); INSERT INTO scratch VALUES (zeroblob(1048576)); DROP TABLE scratch;").unwrap();
        let free = value(
            &mut conn,
            "SELECT freelist_count AS value FROM pragma_freelist_count",
        );
        assert!(free > 0);
        remove_legacy_sourcebans(&mut conn).unwrap();
        assert_eq!(
            value(
                &mut conn,
                "SELECT freelist_count AS value FROM pragma_freelist_count"
            ),
            free
        );
        assert_eq!(
            value(
                &mut conn,
                "SELECT count(*) AS value FROM pragma_integrity_check WHERE integrity_check != 'ok'"
            ),
            0
        );
        drop(conn);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn cleanup_retries_pending_compaction_and_supports_fresh_databases() {
        let mut conn = SqliteConnection::establish(":memory:").unwrap();
        remove_legacy_sourcebans(&mut conn).unwrap();
        setup_schema(&mut conn).unwrap();
        assert_eq!(
            value(
                &mut conn,
                "SELECT count(*) AS value FROM pragma_table_info('account') WHERE name = 'reputation_fetched'"
            ),
            0
        );
        conn.batch_execute("UPDATE app_migrations SET completed = 0;")
            .unwrap();
        remove_legacy_sourcebans(&mut conn).unwrap();
        assert_eq!(
            value(
                &mut conn,
                "SELECT completed AS value FROM app_migrations WHERE name = 'remove_legacy_sourcebans'"
            ),
            1
        );
    }
}
