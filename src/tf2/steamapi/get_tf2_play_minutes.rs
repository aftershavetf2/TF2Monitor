use crate::{models::steamid::SteamID, tf2::lobby::Tf2PlayMinutes};
use reqwest::blocking::get;
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
        steam_api_key, steamid.to_u64()
    );

    let response = get(url);
    match response {
        Ok(response) => {
            match response.json::<Envelope>() {
                Ok(reply) => {
                    // log::info!("Reply: {:?}", reply);
                    match reply
                        .response
                        .games
                        .iter()
                        .find(|g| g.appid == 440)
                        .map(|game| game.playtime_forever)
                    {
                        Some(playtime) => Tf2PlayMinutes::PlayMinutes(playtime),
                        None => Tf2PlayMinutes::Unknown,
                    }
                }
                // The reply was not in the expected format, probably just "{}" because of an private profile
                Err(_) => Tf2PlayMinutes::Unknown,
            }
        }
        Err(e) => {
            log::error!("Error: {}", e);
            Tf2PlayMinutes::Unknown
        }
    }
}
