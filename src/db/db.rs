use sea_orm::{ConnectionTrait, Database, DatabaseConnection, Schema, Statement};
use std::path::Path;

use crate::db::entities::{account, comments, friendship, playtime};

const DATABASE_FILE: &str = "appdata.sqlite3";

/// Connects to the SQLite database and sets up the schema if needed.
///
/// This function:
/// - Connects to the SQLite database file `appdata.db`
/// - Creates the database file if it doesn't exist
/// - Creates all necessary tables based on the entity definitions
/// - Creates indexes as specified in the data model
///
/// # Returns
///
/// Returns a `DatabaseConnection` if successful, or an error if connection/setup fails.
pub async fn connect() -> Result<DatabaseConnection, sea_orm::DbErr> {
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
    let database_url = format!("sqlite://{}?mode=rwc", DATABASE_FILE);
    let db = Database::connect(&database_url).await?;

    log::info!("Connected to database '{}'", DATABASE_FILE);

    // Set up schema (create tables if they don't exist)
    setup_schema(&db).await?;

    Ok(db)
}

/// Sets up the database schema by creating all necessary tables and indexes.
///
/// This function uses SeaORM's Schema API to create tables based on entity definitions.
/// It creates tables if they don't exist, ensuring the database schema matches
/// the current entity definitions (migrations).
async fn setup_schema(db: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    let backend = db.get_database_backend();
    let schema = Schema::new(backend);

    // Create account table if it doesn't exist
    create_table_if_not_exists(db, &schema, account::Entity, "account").await?;

    // Create friendship table if it doesn't exist
    create_table_if_not_exists(db, &schema, friendship::Entity, "friendship").await?;

    // Create comments table if it doesn't exist
    create_table_if_not_exists(db, &schema, comments::Entity, "comments").await?;

    // Create playtime table if it doesn't exist
    create_table_if_not_exists(db, &schema, playtime::Entity, "playtime").await?;

    // Create indexes as specified in DATAMODEL.md
    // Note: Primary keys are automatically indexed, so we only need to create additional indexes

    // Index on Friendship.steam_id (for lookups by account)
    create_index_if_not_exists(db, "idx_friendship_steam_id", "friendship", "steam_id").await?;

    // Index on Friendship.friend_steam_id (for reverse lookups)
    create_index_if_not_exists(
        db,
        "idx_friendship_friend_steam_id",
        "friendship",
        "friend_steam_id",
    )
    .await?;

    // Index on Comments.steam_id (for lookups by account)
    create_index_if_not_exists(db, "idx_comments_steam_id", "comments", "steam_id").await?;

    // Index on Comments.writer_steam_id (for lookups by comment writer)
    create_index_if_not_exists(
        db,
        "idx_comments_writer_steam_id",
        "comments",
        "writer_steam_id",
    )
    .await?;

    // Index on Comments.created_date (for time-based queries)
    create_index_if_not_exists(db, "idx_comments_created_date", "comments", "created_date").await?;

    // Index on Playtime.steam_id (for lookups by account)
    create_index_if_not_exists(db, "idx_playtime_steam_id", "playtime", "steam_id").await?;

    // Index on Playtime.game (for filtering by game)
    create_index_if_not_exists(db, "idx_playtime_game", "playtime", "game").await?;

    log::info!("Database schema setup completed");
    Ok(())
}

/// Creates a table if it doesn't already exist.
///
/// This helper function attempts to create a table and gracefully handles
/// the case where it already exists by catching and ignoring "already exists" errors.
async fn create_table_if_not_exists<T>(
    db: &DatabaseConnection,
    schema: &Schema,
    entity: T,
    table_name: &str,
) -> Result<(), sea_orm::DbErr>
where
    T: sea_orm::EntityTrait,
    <T as sea_orm::EntityTrait>::Model: sea_orm::ModelTrait,
{
    let backend = db.get_database_backend();
    let create_table_stmt = schema.create_table_from_entity(entity);

    match db.execute(backend.build(&create_table_stmt)).await {
        Ok(_) => {
            log::info!("Created '{}' table", table_name);
            Ok(())
        }
        Err(e) => {
            // Check if error is because table already exists
            let err_msg = e.to_string().to_lowercase();
            if err_msg.contains("already exists")
                || err_msg.contains("duplicate")
                || err_msg.contains("table") && err_msg.contains("exists")
            {
                log::debug!("Table '{}' already exists, skipping creation", table_name);
                Ok(())
            } else {
                // Some other error occurred
                Err(e)
            }
        }
    }
}

/// Creates an index if it doesn't already exist.
///
/// This is a helper function that safely creates indexes, avoiding errors
/// if the index already exists.
async fn create_index_if_not_exists(
    db: &DatabaseConnection,
    index_name: &str,
    table_name: &str,
    column_name: &str,
) -> Result<(), sea_orm::DbErr> {
    let backend = db.get_database_backend();

    // SQLite doesn't support CREATE INDEX IF NOT EXISTS directly in all versions,
    // so we'll use a more compatible approach by trying to create it and ignoring errors
    // In practice, SeaORM's create_table_from_entity handles this, but for custom indexes
    // we need to handle it ourselves.

    // For SQLite, we'll use a simple CREATE INDEX statement
    // If the index already exists, SQLite will return an error, which we'll ignore
    let stmt = Statement::from_sql_and_values(
        backend,
        &format!(
            "CREATE INDEX IF NOT EXISTS {} ON {}({})",
            index_name, table_name, column_name
        ),
        [],
    );

    match db.execute(stmt).await {
        Ok(_) => {
            log::debug!(
                "Created index '{}' on {}.{}",
                index_name,
                table_name,
                column_name
            );
            Ok(())
        }
        Err(e) => {
            // If index already exists, that's fine - log and continue
            if e.to_string().contains("already exists") || e.to_string().contains("duplicate") {
                log::debug!("Index '{}' already exists, skipping", index_name);
                Ok(())
            } else {
                Err(e)
            }
        }
    }
}
