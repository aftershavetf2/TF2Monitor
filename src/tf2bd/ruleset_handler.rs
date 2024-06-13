use super::models::{PlayerAttribute, PlayerInfo, RulesFile};
use crate::{
    models::steamid::SteamID,
    tf2::lobby::{PlayerFlag, PlayerMarking},
};
use std::collections::{HashMap, HashSet};

pub struct RulesetHandler {
    pub player_rules: HashMap<SteamID, PlayerMarking>,
    pub source: String,
    pub suggestion: bool,
}

impl RulesetHandler {
    pub fn new(source: &str, suggestion: bool) -> Self {
        let rules_file = RulesFile::load(source);

        let player_rules = get_player_rules(&rules_file, source, suggestion);

        Self {
            player_rules,
            source: source.to_string(),
            suggestion,
        }
    }

    pub fn get_player_marking(&self, steamid: &SteamID) -> Option<&PlayerMarking> {
        self.player_rules.get(steamid)
    }

    pub fn set_player_flags(&mut self, steamid: SteamID, flag: PlayerFlag, enable: bool) {
        if let Some(marking) = self.player_rules.get_mut(&steamid) {
            if enable {
                marking.flags.insert(flag);
            } else {
                marking.flags.remove(&flag);
            }

            if marking.flags.is_empty() {
                self.player_rules.remove(&steamid);
            }
        } else {
            // let mut marking = PlayerMarking {
            //     source: self.source.clone(),
            //     suggestion: self.suggestion,
            //     flags: HashSet::from_iter(vec![flag]),
            // };
            // if enable {
            //     marking.flags.insert(flag);
            // } else {
            //     marking.flags.remove(&flag);
            // }
            // self.player_rules.insert(steamid, marking);

            if enable {
                let mut marking = PlayerMarking {
                    source: self.source.clone(),
                    suggestion: self.suggestion,
                    flags: HashSet::from_iter(vec![flag]),
                };
                marking.flags.insert(flag);
                self.player_rules.insert(steamid, marking);
            }
        }

        self.save();

        // self.player_rules.insert(steamid, marking);
    }

    fn save(&mut self) {
        let mut rules_file = RulesFile::new();
        rules_file.schema = "https://raw.githubusercontent.com/PazerOP/tf2_bot_detector/master/schemas/v3/playerlist.schema.json".to_string();

        for (steamid, marking) in &self.player_rules {
            let player = PlayerInfo {
                steamid32: steamid.to_steam_id32(),
                last_seen: None,
                attributes: marking
                    .flags
                    .iter()
                    .map(|flag| match flag {
                        PlayerFlag::Cheater => PlayerAttribute::Cheater,
                        PlayerFlag::Suspicious => PlayerAttribute::Suspicious,
                        PlayerFlag::Exploiter => PlayerAttribute::Exploiter,
                        PlayerFlag::Toxic => PlayerAttribute::Racist,
                        PlayerFlag::Bot => PlayerAttribute::Bot,
                        PlayerFlag::Cool => PlayerAttribute::Cool,
                    })
                    .collect(),
            };

            if !player.attributes.is_empty() {
                rules_file.players.push(player);
            }
        }

        rules_file.save(&self.source);
    }
}

fn get_player_rules(
    rules_file: &RulesFile,
    source: &str,
    suggestion: bool,
) -> HashMap<SteamID, PlayerMarking> {
    let mut result = HashMap::new();

    for player in &rules_file.players {
        let steamid = SteamID::from_steam_id32(&player.steamid32);
        let marking = get_marking_from_rule(player, source, suggestion);
        result.insert(steamid, marking);
    }

    result
}

fn get_marking_from_rule(rule: &PlayerInfo, source: &str, suggestion: bool) -> PlayerMarking {
    let flags = rule
        .attributes
        .iter()
        .map(|attr| match attr {
            PlayerAttribute::Cheater => PlayerFlag::Cheater,
            PlayerAttribute::Suspicious => PlayerFlag::Suspicious,
            PlayerAttribute::Exploiter => PlayerFlag::Exploiter,
            PlayerAttribute::Racist => PlayerFlag::Toxic,
            PlayerAttribute::Bot => PlayerFlag::Bot,
            PlayerAttribute::Cool => PlayerFlag::Cool,
        })
        .collect();

    PlayerMarking {
        source: source.to_string(),
        suggestion,
        flags,
    }
}
