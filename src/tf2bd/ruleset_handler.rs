use super::models::{PlayerAttribute, PlayerInfo, RulesFile};
use crate::{
    models::steamid::SteamID,
    tf2::lobby::{PlayerFlag, PlayerMarking},
};
use std::collections::HashMap;

pub struct RulesetHandler {
    pub player_rules: HashMap<SteamID, PlayerMarking>,
}

impl RulesetHandler {
    pub fn new(rules_file: &RulesFile, source: &str, suggestion: bool) -> Self {
        let player_rules = get_player_rules(rules_file, source, suggestion);

        Self { player_rules }
    }

    pub fn get_player_marking(&self, steamid: &SteamID) -> Option<&PlayerMarking> {
        self.player_rules.get(steamid)
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
        })
        .collect();

    PlayerMarking {
        source: source.to_string(),
        suggestion,
        flags,
    }
}
