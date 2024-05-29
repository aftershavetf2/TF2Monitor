/*
{
  "players": [
    {
      "SteamId": "76561197974228301",
      "CommunityBanned": false,
      "VACBanned": false,
      "NumberOfVACBans": 0,
      "DaysSinceLastBan": 0,
      "NumberOfGameBans": 0,
      "EconomyBan": "none"
    }
  ]
}
*/

use crate::models::steamid::SteamID;
use reqwest::{blocking::get, Result};
use serde::Deserialize;

use super::SteamPlayerBan;

#[derive(Debug, Deserialize)]
struct PlayerBans {
    #[serde(rename = "SteamId")]
    steamid: String,

    #[serde(rename = "CommunityBanned")]
    community_banned: bool,

    #[serde(rename = "VACBanned")]
    vac_banned: bool,

    #[serde(rename = "NumberOfVACBans")]
    number_of_vac_bans: u32,

    #[serde(rename = "DaysSinceLastBan")]
    days_since_last_ban: u32,

    #[serde(rename = "NumberOfGameBans")]
    number_of_game_bans: u32,

    #[serde(rename = "EconomyBan")]
    economy_ban: String,
}

#[derive(Debug, Deserialize)]
struct Envelope {
    players: Vec<PlayerBans>,
}

pub fn get_bans(steam_api_key: &String, steamids: Vec<SteamID>) -> Option<Vec<SteamPlayerBan>> {
    if steamids.is_empty() {
        return None;
    }

    let steamids: Vec<String> = steamids.iter().map(|s| s.to_u64().to_string()).collect();
    let steamids = steamids.join(",");

    let url = format!(
        "https://api.steampowered.com/ISteamUser/GetPlayerBans/v1/?key={}&steamids={}",
        steam_api_key, steamids
    );

    let response = get(url);
    match response {
        Ok(response) => {
            let reply: Result<Envelope> = response.json();
            match reply {
                Ok(reply) => {
                    let bans: Vec<SteamPlayerBan> = reply
                        .players
                        .iter()
                        .map(|ban| SteamPlayerBan {
                            steamid: SteamID::from_u64_string(&ban.steamid).unwrap_or_default(),
                            community_banned: ban.community_banned,
                            vac_banned: ban.vac_banned,
                            number_of_vac_bans: ban.number_of_vac_bans,
                            days_since_last_ban: ban.days_since_last_ban,
                            number_of_game_bans: ban.number_of_game_bans,
                            economy_ban: ban.economy_ban.clone(),
                        })
                        .collect();

                    Some(bans)
                }
                // The reply was not in the expected format, probably just "{}" because of an private profile
                Err(_) => Some(Vec::new()),
            }
        }
        Err(e) => {
            log::error!("Error: {}", e);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_text() {
        let text = r#"{
            "players": [
              {
                "SteamId": "76561197972495328",
                "CommunityBanned": false,
                "VACBanned": false,
                "NumberOfVACBans": 0,
                "DaysSinceLastBan": 0,
                "NumberOfGameBans": 0,
                "EconomyBan": "none"
              }
            ]
          }"#;

        let p: Envelope = serde_json::from_str(text).unwrap();
        assert_eq!(p.players.len(), 1);
        assert_eq!(p.players[0].steamid, "76561197972495328");
        assert_eq!(p.players[0].community_banned, false);
        assert_eq!(p.players[0].vac_banned, false);
        assert_eq!(p.players[0].number_of_vac_bans, 0);
        assert_eq!(p.players[0].days_since_last_ban, 0);
        assert_eq!(p.players[0].number_of_game_bans, 0);
        assert_eq!(p.players[0].economy_ban, "none");
    }
}
