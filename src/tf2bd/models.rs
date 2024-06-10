//
// Data models for the TF2BD rules file format.
// https://raw.githubusercontent.com/PazerOP/tf2_bot_detector/master/schemas/v3/settings.schema.json
//

use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug)]
pub struct RulesFile {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub file_info: Option<FileInfo>,
    pub rules: Option<Vec<Rule>>,
    pub players: Vec<PlayerInfo>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct PlayerLastSeen {
    pub player_name: Option<String>,
    pub time: u64,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct PlayerInfo {
    pub attributes: Vec<PlayerAttribute>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_seen: Option<PlayerLastSeen>,

    #[serde(rename = "steamid")]
    pub steamid32: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FileInfo {
    authors: Vec<String>,
    description: String,
    title: String,
    update_url: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Rule {
    actions: RuleAction,
    description: String,
    triggers: Trigger,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Trigger {
    #[serde(default)]
    mode: TriggerMode,
    username_text_match: Option<TextMatch>,
    chatmsg_text_match: Option<TextMatch>,
    avatar_match: Option<Vec<AvatarMatch>>,
}

#[derive(Default, Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TriggerMode {
    #[default]
    MatchAll,
    MatchAny,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct TextMatch {
    case_sensitive: bool,
    mode: TextMatchMode,
    patterns: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct AvatarMatch {
    avatar_hash: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "snake_case")]
pub enum TextMatchMode {
    Equal,
    Contains,
    StartsWith,
    EndsWith,
    Regex,
    Word,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RuleAction {
    #[serde(default)]
    mark: Vec<PlayerAttribute>,
    #[serde(default)]
    unmark: Vec<PlayerAttribute>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum PlayerAttribute {
    Cheater,
    Suspicious,
    Exploiter,
    Racist,
    Bot,
    Cool,
}

#[derive(Debug)]
struct RulesStats {
    pub rules: usize,

    pub players: usize,

    pub cheaters: usize,
    pub susicious: usize,
    pub racists: usize,
    pub exploiters: usize,
}

impl RulesFile {
    pub fn new() -> Self {
        Self {
            schema:  "https://raw.githubusercontent.com/PazerOP/tf2_bot_detector/master/schemas/v3/playerlist.schema.json".to_string(),
            file_info: None,
            rules: None,
            players: Vec::new(),
        }
    }

    pub fn load(filename: &str) -> RulesFile {
        if Path::new(filename).exists() {
            log::info!("Loading TF2BD rules file: {}", filename);
            let mut f = File::open(filename).unwrap();
            let mut json = String::new();
            f.read_to_string(&mut json).unwrap();
            let rules = RulesFile::from_json_str(&json);
            log::info!("Stats: {:?}", rules.get_stats());

            rules
        } else {
            log::info!(
                "File {} does not exist. Creating an empty rules file",
                filename
            );
            RulesFile::new()
        }
    }

    pub fn save(&self, filename: &str) {
        log::info!("Saving TF2BD rules file: {}", filename);
        let json = serde_json::to_string_pretty(self).unwrap();
        let mut f = File::create(filename).unwrap();
        f.write_all(json.as_bytes()).unwrap();
        log::info!("Saving done");
    }

    pub fn from_json_str(json: &str) -> RulesFile {
        serde_json::from_str(json).unwrap()
    }

    fn get_stats(&self) -> RulesStats {
        let mut result = RulesStats {
            rules: 0,
            players: self.players.len(),
            cheaters: 0,
            susicious: 0,
            racists: 0,
            exploiters: 0,
        };

        if let Some(rules) = &self.rules {
            result.rules = rules.len();
        }

        result.players = self.players.len();

        for player in &self.players {
            if player.attributes.contains(&PlayerAttribute::Cheater) {
                result.cheaters += 1;
            }
            if player.attributes.contains(&PlayerAttribute::Suspicious) {
                result.susicious += 1;
            }
            if player.attributes.contains(&PlayerAttribute::Exploiter) {
                result.exploiters += 1;
            }
            if player.attributes.contains(&PlayerAttribute::Racist) {
                result.racists += 1;
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_json() {
        let json = r#"
        {
            "$schema": "", "file_info": { "authors": [ "" ], "description": "", "title": "", "update_url": "" },
            "rules": [
                {
                    "actions": {
                        "mark": [
                            "cheater"
                        ]
                    },
                    "description": "description",
                    "triggers": {
                        "username_text_match": {
                            "case_sensitive": true,
                            "mode": "contains",
                            "patterns": [
                                "pattern 1",
                                "pattern 2"
                            ]
                        },
                        "avatar_match": [
                            {
                                "avatar_hash": "76c03c7865876dd13dbe4b60aad86150b8fc6233"
                            }
                        ]
                    }
                }
            ],
            "players": [
                {
                    "attributes": [
                        "cheater"
                    ],
                    "last_seen": {
                        "player_name": "(3)HEXATRONIC",
                        "time": 1686592260
                    },
                    "steamid": "[U:1:1532246671]"
                }
           ]
        }"#;
        let rules_file = RulesFile::from_json_str(json);
        let rules = rules_file.rules.unwrap();

        assert_eq!(rules.len(), 1);
        let rule = &rules[0];
        assert_eq!(rule.description, "description");
        assert_eq!(rule.actions.mark.len(), 1);
        assert_eq!(rule.actions.mark[0], PlayerAttribute::Cheater);
        assert_eq!(rule.triggers.mode, TriggerMode::MatchAll);
        assert_eq!(
            rule.triggers.avatar_match,
            Some(vec![AvatarMatch {
                avatar_hash: "76c03c7865876dd13dbe4b60aad86150b8fc6233".to_string()
            }])
        );
        assert!(rule.triggers.chatmsg_text_match.is_none());
        assert_eq!(
            rule.triggers.username_text_match,
            Some(TextMatch {
                mode: TextMatchMode::Contains,
                case_sensitive: true,
                patterns: vec!["pattern 1".to_string(), "pattern 2".to_string()]
            })
        );

        assert!(rules_file.players.len() == 1);
        assert!(rules_file.players[0].steamid32 == "[U:1:1532246671]".to_string());
    }
}
