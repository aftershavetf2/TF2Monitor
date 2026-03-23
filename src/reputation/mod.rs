use crate::models::steamid::SteamID;
use sourcebans::{SourceBan, SourceBanFetchResult};

pub mod etf2l;
pub mod reputation_thread;
pub mod sourcebans;

#[derive(Debug, Clone)]
pub struct Reputation {
    pub steamid: SteamID,
    pub has_bad_reputation: bool,
    pub source_bans: Vec<SourceBan>,
}

pub fn get_reputation(steamid: SteamID) -> Option<Reputation> {
    let SourceBanFetchResult {
        bans: source_bans,
        successful_sources,
        ..
    } = sourcebans::get_source_bans(steamid);

    if successful_sources == 0 {
        return None;
    }

    Some(Reputation {
        steamid,
        has_bad_reputation: !source_bans.is_empty(),
        source_bans,
    })
}
