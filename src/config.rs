use std::time::Duration;

/// Sleep times used throughout the application

/// RCON thread delays
pub const RCON_DELAY: Duration = Duration::from_millis(100);
pub const RCON_LOOP_DELAY: Duration = Duration::from_millis(2000);

/// Lobby thread delay
pub const LOBBY_LOOP_DELAY: Duration = Duration::from_millis(500);

/// Steam API thread delays
pub const STEAMAPI_LOOP_DELAY: Duration = Duration::from_millis(500);
pub const STEAMAPI_RETRY_DELAY: Duration = Duration::from_millis(5000);

/// Reputation thread delay
pub const REPUTATION_LOOP_DELAY: Duration = Duration::from_millis(100);

/// Logfile watcher delays
pub const LOGFILE_LOOP_DELAY: Duration = Duration::from_millis(1000);
pub const LOGFILE_FILE_NOT_EXIST_DELAY: Duration = Duration::from_millis(10 * 1000);

/// TF2BD thread delay
pub const TF2BD_LOOP_DELAY: Duration = Duration::from_millis(50);

/// GUI delays
pub const GUI_SLEEP_DELAY: Duration = Duration::from_millis(40);
pub const GUI_REPAINT_DELAY: Duration = Duration::from_millis(100);

/// Steam API batch sizes - how many players to process per loop iteration
pub const NUM_PLAYTIMES_TO_FETCH: usize = 4;
pub const NUM_FRIENDS_TO_FETCH: usize = 4;
pub const NUM_ACCOUNT_AGES_TO_APPROX: usize = 1;
pub const NUM_PROFILE_COMMENTS_TO_FETCH: usize = 2;

/// Reputation thread batch size - how many players to process per loop iteration
pub const NUM_REPUTATIONS_TO_FETCH: usize = 3;

/// HTTP cache configuration
pub const HTTP_CACHE_BASE_DIR: &str = ".http-cache";

/// HTTP cache TTL (Time To Live) in days - how long cached data should be used before fetching new data
pub const HTTP_CACHE_TTL_TF2_PLAYTIME_DAYS: i32 = 30;
pub const HTTP_CACHE_TTL_FRIENDLIST_DAYS: i32 = 30;
pub const HTTP_CACHE_TTL_STEAM_COMMENTS_DAYS: i32 = 30;
pub const HTTP_CACHE_TTL_SOURCEBANS_DAYS: i32 = 30;
