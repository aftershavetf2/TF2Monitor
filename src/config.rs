use std::time::Duration;

/// Sleep times used throughout the application

/// RCON thread delays
pub const RCON_DELAY: Duration = Duration::from_millis(10);
pub const RCON_LOOP_DELAY: Duration = Duration::from_millis(5000);

/// Lobby thread delay
pub const LOBBY_LOOP_DELAY: Duration = Duration::from_millis(1000);

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
pub const GUI_SLEEP_DELAY: Duration = Duration::from_millis(80);
pub const GUI_REPAINT_DELAY: Duration = Duration::from_millis(100);

