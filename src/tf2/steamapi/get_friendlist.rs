use crate::models::steamid::SteamID;
use reqwest::{blocking::get, Result};
use serde::Deserialize;
use std::collections::HashSet;

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

    let response = get(url);
    match response {
        Ok(response) => {
            match response.json::<Response>() {
                Ok(reply) => {
                    let friends = reply.friendslist.friends;
                    let players: HashSet<SteamID> = friends
                        .iter()
                        .filter(|f| f.relationship == "friend")
                        .filter_map(|f| SteamID::from_u64_string(&f.steamid))
                        .collect();

                    Some(players)
                }
                // The reply was not in the expected format, probably just "{}" because of an private profile
                Err(_e) => Some(HashSet::new()),
            }
        }
        // There was a communication error
        Err(_e) => None,
    }
}
