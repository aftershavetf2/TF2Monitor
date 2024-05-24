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

pub fn get_bans(steam_api_key: &String, steamid: SteamID) -> Option<SteamPlayerBan> {
    let url = format!(
        "https://api.steampowered.com/ISteamUser/GetPlayerBans/v1/?key={}&steamids={}]",
        steam_api_key,
        steamid.to_u64()
    );

    let response = get(url);
    match response {
        Ok(response) => {
            let reply: Result<Envelope> = response.json();
            let steamid_str = steamid.to_u64().to_string();
            match reply {
                Ok(reply) => {
                    // log::info!("Reply: {:?}", reply);
                    match reply.players.iter().find(|g| g.steamid == steamid_str) {
                        Some(bans) => Some(SteamPlayerBan {
                            steamid,
                            community_banned: bans.community_banned,
                            vac_banned: bans.vac_banned,
                            number_of_vac_bans: bans.number_of_vac_bans,
                            days_since_last_ban: bans.days_since_last_ban,
                            number_of_game_bans: bans.number_of_game_bans,
                            economy_ban: bans.economy_ban.clone(),
                        }),
                        None => Some(SteamPlayerBan {
                            steamid,
                            community_banned: false,
                            vac_banned: false,
                            number_of_vac_bans: 0,
                            days_since_last_ban: 0,
                            number_of_game_bans: 0,
                            economy_ban: "none".to_string(),
                        }),
                    }
                }
                // The reply was not in the expected format, probably just "{}" because of an private profile
                Err(_) => Some(SteamPlayerBan {
                    steamid,
                    community_banned: false,
                    vac_banned: false,
                    number_of_vac_bans: 0,
                    days_since_last_ban: 0,
                    number_of_game_bans: 0,
                    economy_ban: "none".to_string(),
                }),
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
