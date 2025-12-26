use crate::{http_cache::get_from_cache_or_fetch, models::steamid::SteamID};
use serde::Deserialize;
use std::collections::HashSet;

// Days to keep the cache
const DAYS_TO_KEEP: i32 = 7;

#[derive(Debug, Deserialize)]
struct FriendInfo {
    steamid: String,
    relationship: String,
}

#[derive(Debug, Deserialize)]
struct FriendsObject {
    friends: Vec<FriendInfo>,
}

#[derive(Debug, Deserialize)]
struct Response {
    friendslist: FriendsObject,
}

/// Fetches the friends list of a SteamID
/// If there's some error, None is returned
/// Otherwise a list of SteamIDs is returned, possible an empty list
pub fn get_friendlist(steam_api_key: &String, steamid: SteamID) -> Option<HashSet<SteamID>> {
    let url = format!(
        "https://api.steampowered.com/ISteamUser/GetFriendList/v0001/?key={}&steamid={}&relationship=friend",
        steam_api_key, steamid.to_u64()
    );

    if let Some(data) = get_from_cache_or_fetch(
        "Steam Friendlist",
        &steamid.to_u64().to_string(),
        DAYS_TO_KEEP,
        &url,
    ) {
        if let Ok(reply) = serde_json::from_str::<Response>(&data) {
            let friends = reply.friendslist.friends;
            let players: HashSet<SteamID> = friends
                .iter()
                .filter(|f| f.relationship == "friend")
                .filter_map(|f| SteamID::from_u64_string(&f.steamid))
                .collect();

            Some(players)
        } else {
            // Failed to parse the response, return None
            Some(HashSet::new())
        }
    } else {
        // No data back, return None
        None
    }
}
