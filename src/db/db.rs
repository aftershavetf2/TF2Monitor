use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager, Pool};
use std::path::Path;

const DATABASE_FILE: &str = "appdata.sqlite3";

pub type DbPool = Pool<ConnectionManager<SqliteConnection>>;

/// Connects to the SQLite database and sets up the schema if needed.
///
/// This function:
/// - Connects to the SQLite database file `appdata.sqlite3`
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

    // Connect to SQLite database
    // SQLite will create the file if it doesn't exist
    let database_url = DATABASE_FILE;
    let manager = ConnectionManager::<SqliteConnection>::new(database_url);
    let pool = r2d2::Pool::builder()
        .max_size(5)
        .build(manager)?;

    log::info!("Connected to database '{}'", DATABASE_FILE);

    // Set up schema (create tables if they don't exist)
    let mut conn = pool.get()?;
    setup_schema(&mut conn)?;

    Ok(pool)
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
            playtimes_fetched INTEGER,
            reputation_fetched INTEGER
        )"
    ).execute(conn)?;

    // Add reputation_fetched column if it doesn't exist (for existing databases)
    diesel::sql_query(
        "ALTER TABLE account ADD COLUMN reputation_fetched INTEGER"
    ).execute(conn).ok(); // Ignore error if column already exists

    // Create friendship table
    diesel::sql_query(
        "CREATE TABLE IF NOT EXISTS friendship (
            steam_id INTEGER NOT NULL,
            friend_steam_id INTEGER NOT NULL,
            friend_name TEXT NOT NULL,
            friend_date INTEGER NOT NULL,
            unfriend_date INTEGER,
            PRIMARY KEY (steam_id, friend_steam_id)
        )"
    ).execute(conn)?;

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
        )"
    ).execute(conn)?;

    // Create playtime table
    diesel::sql_query(
        "CREATE TABLE IF NOT EXISTS playtime (
            steam_id INTEGER NOT NULL,
            game TEXT NOT NULL,
            play_minutes INTEGER NOT NULL,
            last_updated INTEGER NOT NULL,
            PRIMARY KEY (steam_id, game)
        )"
    ).execute(conn)?;

    // Create bans table
    diesel::sql_query(
        "CREATE TABLE IF NOT EXISTS bans (
            id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
            steam_id INTEGER NOT NULL,
            source TEXT NOT NULL,
            ban_type TEXT NOT NULL,
            reason TEXT,
            created_date INTEGER NOT NULL,
            expires_date INTEGER,
            permanent INTEGER NOT NULL
        )"
    ).execute(conn)?;

    // Create ban_sources table
    diesel::sql_query(
        "CREATE TABLE IF NOT EXISTS ban_sources (
            name TEXT PRIMARY KEY NOT NULL,
            url TEXT NOT NULL,
            parser TEXT NOT NULL,
            last_checked INTEGER,
            active INTEGER NOT NULL
        )"
    ).execute(conn)?;

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
        )"
    ).execute(conn)?;

    // Create indexes as specified in DATAMODEL.md
    // Note: Primary keys are automatically indexed, so we only need to create additional indexes

    diesel::sql_query("CREATE INDEX IF NOT EXISTS idx_friendship_steam_id ON friendship(steam_id)").execute(conn)?;
    diesel::sql_query("CREATE INDEX IF NOT EXISTS idx_friendship_friend_steam_id ON friendship(friend_steam_id)").execute(conn)?;
    diesel::sql_query("CREATE INDEX IF NOT EXISTS idx_comments_steam_id ON comments(steam_id)").execute(conn)?;
    diesel::sql_query("CREATE INDEX IF NOT EXISTS idx_comments_writer_steam_id ON comments(writer_steam_id)").execute(conn)?;
    diesel::sql_query("CREATE INDEX IF NOT EXISTS idx_comments_created_date ON comments(created_date)").execute(conn)?;
    diesel::sql_query("CREATE INDEX IF NOT EXISTS idx_playtime_steam_id ON playtime(steam_id)").execute(conn)?;
    diesel::sql_query("CREATE INDEX IF NOT EXISTS idx_playtime_game ON playtime(game)").execute(conn)?;
    diesel::sql_query("CREATE INDEX IF NOT EXISTS idx_bans_steam_id ON bans(steam_id)").execute(conn)?;
    diesel::sql_query("CREATE INDEX IF NOT EXISTS idx_bans_source ON bans(source)").execute(conn)?;
    diesel::sql_query("CREATE INDEX IF NOT EXISTS idx_ban_sources_active ON ban_sources(active)").execute(conn)?;
    diesel::sql_query("CREATE INDEX IF NOT EXISTS idx_player_flags_steam_id ON player_flags(steam_id)").execute(conn)?;
    diesel::sql_query("CREATE INDEX IF NOT EXISTS idx_player_flags_notified ON player_flags(notified)").execute(conn)?;

    log::info!("Database schema setup completed");
    Ok(())
}
