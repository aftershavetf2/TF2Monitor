use std::vec;

use crate::models::steamid::SteamID;
use reqwest::{blocking::get, Result};
use serde::Deserialize;

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
pub fn get_friendlist(steam_api_key: &String, steamid: SteamID) -> Option<Vec<SteamID>> {
    let url = format!(
        "https://api.steampowered.com/ISteamUser/GetFriendList/v0001/?key={}&steamid={}&relationship=friend",
        steam_api_key, steamid.to_u64()
    );

    log::debug!(
        "Fetching friends of {} from URL '{}'",
        steamid.to_u64(),
        url
    );

    let response = get(url);
    match response {
        Ok(response) => {
            let reply: Result<Response> = response.json();
            match reply {
                Ok(reply) => {
                    log::debug!("Reply: {:?}", reply);

                    let friends = reply.friendslist.friends;
                    let players: Vec<SteamID> = friends
                        .iter()
                        .filter(|f| f.relationship == "friend")
                        .filter_map(|f| SteamID::from_u64_string(&f.steamid))
                        .collect();

                    Some(players)
                }
                // The reply was not in the expected format, probably just "{}" because of an private profile
                Err(_e) => Some(vec![]),
            }
        }
        // There was a communication error
        Err(_e) => None,
    }
}
