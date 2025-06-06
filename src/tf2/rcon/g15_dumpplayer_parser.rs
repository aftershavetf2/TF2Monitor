use super::G15DumpPlayerOutput;
use crate::{
    models::steamid::{self, SteamID},
    tf2::{lobby::Team, rcon::G15PlayerData},
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub struct G15DumpPlayerParser {
    current_players: HashMap<SteamID, PlayerEntry>,
    default_seen_counter: u32,
}

impl G15DumpPlayerParser {
    pub fn new() -> Self {
        Self {
            current_players: HashMap::new(),
            default_seen_counter: 8,
        }
    }

    pub fn parse(&mut self, data: &str) -> G15DumpPlayerOutput {
        let dump = Self::parse_dump(data, self.default_seen_counter);

        // Add players from the current dump to the current_players map
        for (_, player) in dump.player_resource.players.iter() {
            if !player.is_active() {
                continue;
            }

            if let Some(steamid) = player.steamid {
                self.current_players.insert(steamid, player.clone());
            }
        }

        // Decrease seen_counter for all players in the current_players map and remove players with seen_counter == 0
        let mut players_to_remove = Vec::new();
        for (steamid, player) in self.current_players.iter_mut() {
            player.seen_counter = player.seen_counter.saturating_sub(1);
            if player.seen_counter == 0 {
                players_to_remove.push(*steamid);
            }
        }

        for steamid in players_to_remove {
            self.current_players.remove(&steamid);
        }

        let mut result = G15DumpPlayerOutput::default();

        for (steamid, player) in self.current_players.iter() {
            if !player.is_active() {
                continue;
            }

            // println!("Player #{i}: {:?}", player.name);
            result.players.push(G15PlayerData {
                steamid: *steamid,
                id: player.user_id.unwrap_or(0) as i64,
                name: player.name.clone().unwrap(),
                ping: player.ping.unwrap_or(0) as i64,
                alive: player.alive.unwrap_or(false),
                team: match player.team {
                    Some(2) => Some(Team::Red),
                    Some(3) => Some(Team::Blue),
                    _ => None,
                },
                score: player.score.unwrap_or(0) as i64,
            });
        }

        result
    }

    fn parse_dump(data: &str, default_seen_counter: u32) -> DumpData {
        let mut dump = DumpData::default();
        let mut current_section = "";

        let re_field = Regex::new(r"(?m)^([a-zA-Z0-9_\[\]]+)\s+\w+\s+\((.*?)\)$").unwrap();
        let re_indexed = Regex::new(r"([^\[]+)\[(\d+)\]").unwrap();

        for line in data.lines() {
            if line.starts_with("(playerresource)") {
                current_section = "playerresource";
                continue;
            }

            if current_section == "playerresource" {
                if let Some(caps) = re_field.captures(line) {
                    let raw_key = &caps[1];
                    let value_str = &caps[2];

                    if let Some(cap_indexed) = re_indexed.captures(raw_key) {
                        let key = cap_indexed[1].to_string();
                        let index: usize = cap_indexed[2].parse().unwrap_or(999);

                        let entry = dump.player_resource.players.entry(index).or_default();

                        match key.as_str() {
                            "m_szName" => entry.name = Some(value_str.to_string()),
                            "m_bConnected" => entry.connected = Some(value_str == "true"),
                            "m_bValid" => entry.valid = Some(value_str == "true"),
                            "m_bAlive" => entry.alive = Some(value_str == "true"),
                            "m_iHealth" => entry.health = value_str.parse().ok(),
                            "m_iPing" => entry.ping = value_str.parse().ok(),
                            "m_iScore" => entry.score = value_str.parse().ok(),
                            "m_iDeaths" => entry.deaths = value_str.parse().ok(),
                            "m_iTeam" => entry.team = value_str.parse().ok(),
                            "m_iAccountID" => entry.account_id = value_str.parse().ok(),
                            "m_iUserID" => entry.user_id = value_str.parse().ok(),
                            _ => {}
                        }
                    }
                }
            }
        }

        // Set seen_counter for all players
        for (_, player) in dump.player_resource.players.iter_mut() {
            player.seen_counter = default_seen_counter;
            if let Some(account_id) = player.account_id {
                player.steamid = Some(steamid::SteamID::from_u64(
                    account_id as u64 + steamid::MIN_STEAMID64,
                ));
            }
        }

        dump
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
struct PlayerEntry {
    pub name: Option<String>,
    pub connected: Option<bool>,
    pub valid: Option<bool>,
    pub alive: Option<bool>,
    pub health: Option<u32>,
    pub ping: Option<u32>,
    pub score: Option<u32>,
    pub deaths: Option<u32>,
    pub team: Option<u32>,
    pub account_id: Option<u32>,
    pub user_id: Option<u32>,
    pub seen_counter: u32,
    pub steamid: Option<SteamID>,
}

impl PlayerEntry {
    pub fn is_active(&self) -> bool {
        self.connected == Some(true)
            && self.valid == Some(true)
            && self.name.is_some()
            //&& self.name.as_deref() != Some("unconnected")
            && self.account_id.unwrap_or(0) != 0
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct PlayerResource {
    pub players: HashMap<usize, PlayerEntry>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct DumpData {
    pub player_resource: PlayerResource,
}

#[cfg(test)]
mod tests {
    // use chrono::prelude::*;

    // use steamid::SteamID;

    // use super::*;

    // fn get_dump_text() -> String {
    //     let bytes = include_bytes!("g15_dumpplayer_output.txt");
    //     let s = String::from_utf8_lossy(bytes);
    //     s.to_string()
    // }

    // #[test]
    // fn test_parse() {
    //     let dump = get_dump_text();
    //     let output = parse_g15_dump(&dump);

    //     for (i, player) in output.players.iter().enumerate() {
    //         println!("{}: {:?}", i, player);
    //     }

    //     assert_eq!(19, output.players.len());
    //     let player = &output.players[15];
    //     assert_eq!("aftershave".to_string(), player.name);
    //     assert_eq!(SteamID::from_u64(76561197974228301), player.steamid);
    //     assert_eq!(23, output.players[15].ping);
    // }

    // Without the filter in parse() this took 17 seconds.
    // With the filter in parse() it takes 10 seconds.
    // With a .start_with(prefix) it takes 5 seconds.
    // #[test]
    // fn test_bench_parse() {
    //     let dump = get_dump_text();

    //     let start_time = std::time::Instant::now();
    //     for _ in 0..1000 {
    //         let _output = parse_g15_dump(&dump);
    //     }
    //     let stop_time = std::time::Instant::now();
    //     let elapsed = stop_time - start_time;
    //     println!("Elapsed: {:?}", elapsed);

    //     assert!(false);
    // }
}
