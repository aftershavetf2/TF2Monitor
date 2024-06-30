pub mod models;
pub mod ruleset_handler;
pub mod tf2bd_thread;

use crate::models::steamid::SteamID;
use models::PlayerInfo;

#[derive(Debug, Clone)]
pub enum Tf2bdMsg {
    /// SteamID, PlayerMarking
    Tf2bdPlayerMarking(SteamID, Option<PlayerInfo>),
}
