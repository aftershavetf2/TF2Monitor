use crate::models::steamid::SteamID;
use reqwest::blocking::get;
use serde::Deserialize;

use super::SteamApiPlayer;

#[derive(Deserialize)]
struct Players {
    players: Vec<SteamApiPlayer>,
}

#[derive(Deserialize)]
struct GetPlayerSummariesApiResponse {
    response: Players,
}

pub fn get_player_summaries(
    steam_api_key: &String,
    steamids: Vec<SteamID>,
) -> Option<Vec<SteamApiPlayer>> {
    if steamids.is_empty() {
        return None;
    }

    let steamids: Vec<String> = steamids.iter().map(|s| s.to_u64().to_string()).collect();
    let steamids = steamids.join(",");

    let url = format!(
        "https://api.steampowered.com/ISteamUser/GetPlayerSummaries/v2/?key={}&steamids={}",
        steam_api_key, steamids
    );

    let response = get(&url);
    match response {
        Ok(response) => match response.json::<GetPlayerSummariesApiResponse>() {
            Ok(response) => {
                let players = response.response.players;
                Some(players)
            }
            Err(e) => {
                log::error!("Error parsing player summaries: {}. URL: {}", e, url);
                None
            }
        },
        Err(e) => {
            log::error!("Error fetching player summaries: {}", e);
            None
        }
    }
}
