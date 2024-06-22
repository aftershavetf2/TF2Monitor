use super::LogLine;
use chrono::{offset::LocalResult, prelude::*};
use regex::Regex;

const TIMESTAMP_LEN: usize = 23;

pub struct LogLineParser {
    killed_rx: Regex,
    lobby_debug_rx: Regex,
    suicided_rx: Regex,
    player_status_rx: Regex,
    chat_rx: Regex,
}

impl Default for LogLineParser {
    fn default() -> Self {
        Self::new()
    }
}

impl LogLineParser {
    pub fn new() -> Self {
        let player_statys_str = {
            let id = r"\d{1,6}";
            let name = r".+?";
            let steamid32 = r"\[U:\d:\d{1,10}]";
            let time = r"\d{1,3}:\d{2}(:\d{2})?";
            let ping = r"\d{1,4}";
            let loss = r"\d{1,3}";
            let state = r"(spawning|active)";
            format!(
                r#"^#\s{{1,6}}({id}) "({name})"\s+({steamid32})\s{{1,9}}{time}\s+{ping}\s{{1,8}}{loss} {state}$"#
            )
        };

        let lobby_debug_rx = r#"^\s{2}(Member|Pending)\[\d+]\s+(?P<sid>\[.+?]).+?TF_GC_TEAM_(?P<team>(DEFENDERS|INVADERS))\s{2}type\s=\sMATCH_PLAYER$"#;

        Self {
            killed_rx: Regex::new(r"^(.+?) killed (.+?) with (.+)(\.|\. \(crit\))$").unwrap(),
            suicided_rx: Regex::new(r"^(.+?) suicided.$").unwrap(),
            player_status_rx: Regex::new(player_statys_str.as_str()).unwrap(),
            lobby_debug_rx: Regex::new(lobby_debug_rx).unwrap(),
            chat_rx: Regex::new(r"^(.+?) :  (.+)$").unwrap(),
        }
    }

    pub fn parse_line(&self, org_line: &str) -> Option<LogLine> {
        let when = self.try_get_when(org_line);
        if when.is_some() {
            let when = when.unwrap();
            let line = &org_line[TIMESTAMP_LEN..];

            let logobj = self.parse_tf_lobby_debug_line(when, line);
            if logobj.is_some() {
                return logobj;
            }

            let logobj = self.parse_killed_line(when, line);
            if logobj.is_some() {
                return logobj;
            }

            let logobj = self.parse_status_header_line(when, line);
            if logobj.is_some() {
                return logobj;
            }

            let logobj = self.parse_status_player_line(when, line);
            if logobj.is_some() {
                return logobj;
            }

            let logobj = self.parse_lobby_status_line(when, line);
            if logobj.is_some() {
                return logobj;
            }

            let logobj = self.parse_chat_line(when, line);
            if logobj.is_some() {
                return logobj;
            }

            let logobj = self.parse_suicided_line(when, line);
            if logobj.is_some() {
                return logobj;
            }
        }

        None
    }

    pub fn parse_killed_line(&self, when: DateTime<Local>, line: &str) -> Option<LogLine> {
        let caps = self.killed_rx.captures(line);

        match caps {
            Some(caps) => {
                let killer = caps.get(1).unwrap().as_str().to_string();
                let victim = caps.get(2).unwrap().as_str().to_string();
                let weapon = caps.get(3).unwrap().as_str().to_string();
                let crit = caps.get(4).unwrap().as_str().ends_with(". (crit)");

                Some(LogLine::Kill {
                    when,
                    killer,
                    victim,
                    weapon,
                    crit,
                })
            }
            None => None,
        }
    }

    pub fn parse_suicided_line(&self, when: DateTime<Local>, line: &str) -> Option<LogLine> {
        let caps = self.suicided_rx.captures(line);

        match caps {
            Some(caps) => {
                let name = caps.get(1).unwrap().as_str().to_string();
                Some(LogLine::Suicide { when, name })
            }
            None => None,
        }
    }

    pub fn parse_status_player_line(&self, when: DateTime<Local>, line: &str) -> Option<LogLine> {
        let caps = self.player_status_rx.captures(line);
        match caps {
            Some(caps) => {
                let id: u32 = caps.get(1).unwrap().as_str().to_string().parse().unwrap();
                let name = caps.get(2).unwrap().as_str().to_string();
                let steam_id32 = caps.get(3).unwrap().as_str().to_string();
                Some(LogLine::StatusForPlayer {
                    when,
                    id,
                    name,
                    steam_id32,
                })
            }
            None => None,
        }
    }

    pub fn parse_lobby_status_line(&self, when: DateTime<Local>, line: &str) -> Option<LogLine> {
        if line == "Lobby created" {
            return Some(LogLine::LobbyCreated { when });
        }

        if line == "Lobby destroyed" {
            return Some(LogLine::LobbyDestroyed { when });
        }

        None
    }

    pub fn parse_chat_line(&self, when: DateTime<Local>, line: &str) -> Option<LogLine> {
        let caps = self.chat_rx.captures(line);
        match caps {
            Some(caps) => {
                let mut dead = false;
                let mut team = false;

                let mut name = caps.get(1).unwrap().as_str().to_string();
                let message = caps.get(2).unwrap().as_str().to_string();

                if name.starts_with("*DEAD*(TEAM) ") {
                    name = name.trim_start_matches("*DEAD*(TEAM) ").to_string();
                    dead = true;
                    team = true;
                } else if name.starts_with("*DEAD* ") {
                    name = name.trim_start_matches("*DEAD* ").to_string();
                    dead = true;
                } else if name.starts_with("(TEAM) ") {
                    name = name.trim_start_matches("(TEAM) ").to_string();
                    team = true;
                }

                Some(LogLine::Chat {
                    when,
                    name,
                    message,
                    dead,
                    team,
                })
            }
            None => None,
        }
    }

    pub fn parse_status_header_line(&self, when: DateTime<Local>, line: &str) -> Option<LogLine> {
        const HEADER: &str =
            "# userid name                uniqueid            connected ping loss state";
        if line == HEADER {
            return Some(LogLine::StatusHeader { when });
        }

        None
    }

    pub fn parse_tf_lobby_debug_line(&self, _when: DateTime<Local>, line: &str) -> Option<LogLine> {
        let caps = self.lobby_debug_rx.captures(line);

        match caps {
            Some(caps) => {
                let steam_id32 = caps["sid"].to_string();
                let team = caps["team"].to_string().to_string();

                Some(LogLine::PlayerTeam { steam_id32, team })
            }
            None => None,
        }
    }

    pub fn try_get_when(&self, line: &str) -> Option<DateTime<Local>> {
        if line.len() < TIMESTAMP_LEN {
            return None;
        }

        let fmt = "%m/%d/%Y - %H:%M:%S";

        let line = &line[0..21];
        let result = NaiveDateTime::parse_from_str(line, fmt);

        match result {
            Ok(when) => match Local.from_local_datetime(&when) {
                LocalResult::Single(when) => Some(when),
                LocalResult::Ambiguous(_e, _l) => {
                    // println!("Ambiguous: Error parsing date: {:?}, {:?}", e, l);
                    None
                }
                LocalResult::None => {
                    // println!("None: Error parsing date");
                    None
                }
            },
            Err(_e) => {
                // println!("Error parsing date: {:?}. Line = '{}'", e, line);
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::prelude::*;

    use super::*;

    #[test]
    fn test_get_date() {
        let parser = LogLineParser::default();

        let when = Local.with_ymd_and_hms(2020, 11, 7, 08, 41, 39).unwrap();

        let result = parser.try_get_when("11/07/2020 - 08:41:39: #");
        assert_eq!(result, Some(when));

        let result = parser.try_get_when("11/07/2020 - 08:41:39");
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_kill_line() {
        let parser = LogLineParser::default();

        let when = Local.with_ymd_and_hms(2024, 5, 6, 17, 2, 55).unwrap();
        let line = "05/06/2024 - 17:02:55: Player1 killed Player2 with iron_bomber. (crit)";
        let result = parser.parse_line(line).unwrap();
        assert_eq!(
            result,
            LogLine::Kill {
                when,
                killer: "Player1".to_string(),
                victim: "Player2".to_string(),
                weapon: "iron_bomber".to_string(),
                crit: true,
            }
        );

        let line = "05/06/2024 - 17:02:55: Player1 killed Player2 with iron_bomber.";
        let result = parser.parse_line(line).unwrap();
        assert_eq!(
            result,
            LogLine::Kill {
                when,
                killer: "Player1".to_string(),
                victim: "Player2".to_string(),
                weapon: "iron_bomber".to_string(),
                crit: false,
            }
        );

        let line = "05/06/2024 - 17:02:55: 𝖁𝖆𝖘𝖎𝖑𝖎𝖘� killed rafailnn306 with syringegun_medic.";
        let result = parser.parse_line(line).unwrap();
        assert_eq!(
            result,
            LogLine::Kill {
                when,
                killer: "𝖁𝖆𝖘𝖎𝖑𝖎𝖘�".to_string(),
                victim: "rafailnn306".to_string(),
                weapon: "syringegun_medic".to_string(),
                crit: false,
            }
        );
    }

    #[test]
    fn test_parse_suicided_line() {
        let parser = LogLineParser::default();

        let when = Local.with_ymd_and_hms(2024, 5, 8, 13, 30, 42).unwrap();
        let line = "05/08/2024 - 13:30:42: Player1 suicided.";
        let result = parser.parse_line(line).unwrap();
        assert_eq!(
            result,
            LogLine::Suicide {
                when,
                name: "Player1".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_status_player_line() {
        let parser = LogLineParser::default();

        let when = Local.with_ymd_and_hms(2024, 5, 8, 13, 30, 42).unwrap();
        let line = r#"05/08/2024 - 13:30:42: #   1371 "Player1"           [U:1:169802]     10:03       85    0 active"#;
        let result = parser.parse_line(line).unwrap();
        assert_eq!(
            result,
            LogLine::StatusForPlayer {
                when,
                id: 1371,
                name: "Player1".to_string(),
                steam_id32: "[U:1:169802]".to_string()
            }
        );
    }

    #[test]
    fn test_parse_status_player_line2() {
        let parser = LogLineParser::default();

        let when = Local.with_ymd_and_hms(2024, 6, 22, 12, 12, 31).unwrap();
        let line = r#"#   1580 "Red."              [U:1:53518]         09:51       74    0 active"#;
        let result = parser.parse_status_player_line(when, line).unwrap();
        assert_eq!(
            result,
            LogLine::StatusForPlayer {
                when,
                id: 1580,
                name: "Red.".to_string(),
                steam_id32: "[U:1:53518]".to_string()
            }
        );
    }

    #[test]
    fn test_parse_lobby_status_line() {
        let parser = LogLineParser::default();

        // 05/08/2024 - 13:30:42: Lobby created
        // 05/08/2024 - 13:30:42: Lobby destroyed

        let when = Local.with_ymd_and_hms(2024, 5, 8, 13, 30, 42).unwrap();
        let line = r#"05/08/2024 - 13:30:42: Lobby created"#;
        let result = parser.parse_line(line).unwrap();
        assert_eq!(result, LogLine::LobbyCreated { when });

        let line = r#"05/08/2024 - 13:30:42: Lobby destroyed"#;
        let result = parser.parse_line(line).unwrap();
        assert_eq!(result, LogLine::LobbyDestroyed { when });
    }

    #[test]
    fn test_parse_chat_line() {
        let parser = LogLineParser::default();

        let when = Local.with_ymd_and_hms(2024, 5, 8, 13, 30, 42).unwrap();

        let line = "05/08/2024 - 13:30:42: Player1 :  hello";
        let result = parser.parse_line(line).unwrap();
        assert_eq!(
            result,
            LogLine::Chat {
                when,
                name: "Player1".to_string(),
                message: "hello".to_string(),
                dead: false,
                team: false,
            }
        );

        let line = "05/08/2024 - 13:30:42: (TEAM) Player1 :  hello again";
        let result = parser.parse_line(line).unwrap();
        assert_eq!(
            result,
            LogLine::Chat {
                when,
                name: "Player1".to_string(),
                message: "hello again".to_string(),
                dead: false,
                team: true,
            }
        );

        let line = "05/08/2024 - 13:30:42: *DEAD* Player1 :  hello dead";
        let result = parser.parse_line(line).unwrap();
        assert_eq!(
            result,
            LogLine::Chat {
                when,
                name: "Player1".to_string(),
                message: "hello dead".to_string(),
                dead: true,
                team: false,
            }
        );

        let line = "05/08/2024 - 13:30:42: *DEAD*(TEAM) Player1 :  hello again dead";
        let result = parser.parse_line(line).unwrap();
        assert_eq!(
            result,
            LogLine::Chat {
                when,
                name: "Player1".to_string(),
                message: "hello again dead".to_string(),
                dead: true,
                team: true,
            }
        );
    }

    #[test]
    fn test_parse_status_header() {
        let parser = LogLineParser::default();

        let line = r#"05/08/2024 - 14:25:11: # userid name                uniqueid            connected ping loss state"#;
        let result = parser.parse_line(line).unwrap();
        assert_eq!(
            result,
            LogLine::StatusHeader {
                when: Local.with_ymd_and_hms(2024, 5, 8, 14, 25, 11).unwrap(),
            }
        );
    }

    #[test]
    fn test_parse_tf_lobby_player_header() {
        let parser = LogLineParser::default();

        let line = r#"05/08/2024 - 14:25:11:   Member[23] [U:1:169802]  team = TF_GC_TEAM_DEFENDERS  type = MATCH_PLAYER"#;
        let result = parser.parse_line(line).unwrap();
        assert_eq!(
            result,
            LogLine::PlayerTeam {
                steam_id32: "[U:1:169802]".to_string(),
                team: "DEFENDERS".to_string(),
            }
        );

        let line = r#"05/08/2024 - 14:25:11:   Member[23] [U:1:169802]  team = TF_GC_TEAM_INVADERS  type = MATCH_PLAYER"#;
        let result = parser.parse_line(line).unwrap();
        assert_eq!(
            result,
            LogLine::PlayerTeam {
                steam_id32: "[U:1:169802]".to_string(),
                team: "INVADERS".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_order() {
        let parser = LogLineParser::default();

        let when = Local.with_ymd_and_hms(2024, 5, 8, 13, 30, 42).unwrap();
        let line = r#"05/08/2024 - 13:30:42: Player1 suicided."#;
        let result = parser.parse_line(line).unwrap();
        assert_eq!(
            result,
            LogLine::Suicide {
                when,
                name: "Player1".to_string()
            }
        );

        let line = r#"05/08/2024 - 13:30:42: Player1 :  suicided."#;
        let result = parser.parse_line(line).unwrap();
        assert_eq!(
            result,
            LogLine::Chat {
                when,
                name: "Player1".to_string(),
                message: "suicided.".to_string(),
                dead: false,
                team: false
            }
        );
    }

    #[test]
    fn test_parse_datetime() {
        let fmt = "%m/%d/%Y - %H:%M:%S";

        let line = "05/08/2024 - 13:30:42";
        let actual = NaiveDateTime::parse_from_str(line, fmt)
            .ok()
            .map(|dt| Local.from_local_datetime(&dt).unwrap());

        let expected = Local.with_ymd_and_hms(2024, 5, 8, 13, 30, 42).unwrap();

        assert_eq!(actual, Some(expected));
    }
}
