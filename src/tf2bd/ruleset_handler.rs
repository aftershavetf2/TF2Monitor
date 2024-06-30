use chrono::Local;

use super::models::{PlayerAttribute, PlayerInfo, PlayerLastSeen, TF2BDPlayerList};
use crate::{models::steamid::SteamID, tf2::lobby::Player};
use std::collections::HashMap;

pub struct RulesetHandler {
    pub source: String,
    // pub suggestion: bool,
    pub tf2bd_playerlist: TF2BDPlayerList,

    pub player_rules: HashMap<SteamID, PlayerInfo>,
}

impl RulesetHandler {
    pub fn new(source: &str, _suggestion: bool) -> Self {
        let rules_file = TF2BDPlayerList::load(source);

        let player_rules = get_player_rules(&rules_file);

        Self {
            source: source.to_string(),
            //suggestion,
            tf2bd_playerlist: rules_file,
            player_rules,
        }
    }

    pub fn get_player_marking(&self, steamid: &SteamID) -> Option<&PlayerInfo> {
        self.player_rules.get(steamid)
    }

    pub fn set_player_flags(&mut self, player: Player, flag: PlayerAttribute, enable: bool) {
        if let Some(player_info) = self.player_rules.get_mut(&player.steamid) {
            player_info.attributes.retain(|x| *x != flag);
            if enable {
                player_info.attributes.push(flag);
            }

            if player_info.attributes.is_empty() {
                self.player_rules.remove(&player.steamid);
            }
        } else if enable {
            let last_seen = PlayerLastSeen {
                player_name: Some(player.name.clone()),
                time: Local::now().timestamp(),
            };

            let player_info = PlayerInfo {
                steamid32: player.steamid.to_steam_id32(),
                last_seen: Some(last_seen),
                attributes: vec![flag],
            };

            self.player_rules.insert(player.steamid, player_info);
        }

        self.save();

        // self.player_rules.insert(steamid, marking);
    }

    fn save(&mut self) {
        let mut rules_file = self.tf2bd_playerlist.clone();
        rules_file.schema = "https://raw.githubusercontent.com/PazerOP/tf2_bot_detector/master/schemas/v3/playerlist.schema.json".to_string();

        rules_file.players.clear();

        for player_info in self.player_rules.values() {
            if !player_info.attributes.is_empty() {
                rules_file.players.push(player_info.clone());
            }
        }

        rules_file.save(&self.source);
    }
}

fn get_player_rules(rules_file: &TF2BDPlayerList) -> HashMap<SteamID, PlayerInfo> {
    let mut result = HashMap::new();

    for player_info in &rules_file.players {
        let steamid = SteamID::from_steam_id32(&player_info.steamid32);
        result.insert(steamid, player_info.clone());
    }

    result
}
