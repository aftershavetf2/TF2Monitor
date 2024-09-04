use crate::models::steamid::SteamID;

use super::lobby::Team;

pub mod g15_dumpplayer_parser;
pub mod rcon_connection;
pub mod rcon_thread;

#[derive(Debug, Clone, Default)]
pub struct G15DumpPlayerOutput {
    pub players: Vec<G15PlayerData>,
}

#[derive(Debug, Clone, Default)]
pub struct G15PlayerData {
    pub steamid: SteamID,
    pub id: i64,
    pub name: String,
    pub ping: i64,
    pub alive: bool,
    pub team: Option<Team>,
    pub score: i64,
}
