use crate::models::steamid::SteamID;
use sourcebans::SourceBan;

pub mod reputation_thread;
pub mod sourcebans;

#[derive(Debug, Clone)]
pub struct Reputation {
    pub steamid: SteamID,
    pub has_bad_reputation: bool,
    pub bans: Vec<SourceBan>,
}

pub fn get_reputation(steamid: SteamID) -> Reputation {
    let bans = sourcebans::get_source_bans(steamid);
    let has_bad_reputation = !bans.is_empty();

    Reputation {
        steamid,
        has_bad_reputation,
        bans,
    }
}
