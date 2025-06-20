use std::error::Error;

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
) -> Result<Vec<SteamApiPlayer>, Box<dyn Error>> {
    let steamids: Vec<String> = steamids.iter().map(|s| s.to_u64().to_string()).collect();
    let steamids = steamids.join(",");

    let url = format!(
        "https://api.steampowered.com/ISteamUser/GetPlayerSummaries/v2/?key={}&steamids={}",
        steam_api_key, steamids
    );

    let response = get(&url)?;
    let players = response.json::<GetPlayerSummariesApiResponse>()?;

    Ok(players.response.players)
}
