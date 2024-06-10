pub mod models;
pub mod ruleset_handler;
pub mod tf2bd_thread;

use crate::{models::steamid::SteamID, tf2::lobby::PlayerMarking};

#[derive(Debug, Clone)]
pub enum Tf2bdMsg {
    Tf2bdPlayerMarking(SteamID, PlayerMarking),
}
