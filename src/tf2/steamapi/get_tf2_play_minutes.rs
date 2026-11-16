use crate::{models::steamid::SteamID, tf2::lobby::Tf2PlayMinutes};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Game {
    appid: u32,
    playtime_forever: u32,
}

#[derive(Debug, Deserialize)]
struct Payload {
    games: Vec<Game>,
}

#[derive(Debug, Deserialize)]
struct Envelope {
    response: Payload,
}

pub fn get_tf2_play_minutes(steam_api_key: &String, steamid: SteamID) -> Tf2PlayMinutes {
    let url = format!(
        "http://api.steampowered.com/IPlayerService/GetRecentlyPlayedGames/v0001/?key={}&steamid={}&count=50&format=json",
        steam_api_key,
        steamid.to_u64()
    );

    if let Ok(data) = reqwest::blocking::get(&url).and_then(|response| response.text())
        && let Ok(reply) = serde_json::from_str::<Envelope>(&data)
    {
        match reply
            .response
            .games
            .iter()
            .find(|g| g.appid == 440)
            .map(|game| game.playtime_forever)
        {
            Some(playtime) => {
                return Tf2PlayMinutes::PlayMinutes(playtime);
            }
            None => {
                return Tf2PlayMinutes::Unknown;
            }
        }
    }

    Tf2PlayMinutes::Unknown
}
