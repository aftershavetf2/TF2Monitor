use super::LogLine;
use regex::Regex;

pub struct LogLineParser {
    timestamped_line_rx: Regex,
    killed_rx: Regex,
    suicided_rx: Regex,
    chat_rx: Regex,
    differing_lobby_rx: Regex,
}

impl Default for LogLineParser {
    fn default() -> Self {
        Self::new()
    }
}

impl LogLineParser {
    pub fn new() -> Self {
        Self {
            timestamped_line_rx: Regex::new(r"^\d{2}/\d{2}/\d{4} - \d{2}:\d{2}:\d{2}: ").unwrap(),
            killed_rx: Regex::new(r"^(.+?) killed (.+) with (.+?)(\.|\. \(crit\))$").unwrap(),
            suicided_rx: Regex::new(r"^(.+?) suicided.$").unwrap(),
            chat_rx: Regex::new(r"^(.+?) :  (.+)$").unwrap(),

            //         // Differing lobby received. Lobby: [A:1:3982432273:50775]/Match117694416/Lobby713780413559521 CurrentlyAssigned: [A:1:3982432273:50775]/Match117693976/Lobby713780413559521

            differing_lobby_rx: Regex::new(r"^Differing lobby received\. Lobby: \[A:1:\d+:\d+\]/Match\d+/Lobby\d+ CurrentlyAssigned: \[A:1:\d+:\d+\]/Match\d+/Lobby\d+$").unwrap(),
        }
    }

    pub fn parse_line(&self, line: &str) -> Option<LogLine> {
        let line = match self.timestamped_line_rx.find(line) {
            Some(timestamp) => &line[timestamp.end()..],
            None => line,
        };

        let logobj = self.parse_killed_line(line);
        if logobj.is_some() {
            return logobj;
        }

        let logobj = self.parse_lobby_status_line(line);
        if logobj.is_some() {
            return logobj;
        }

        let logobj = self.parse_chat_line(line);
        if logobj.is_some() {
            return logobj;
        }

        let logobj = self.parse_suicided_line(line);
        if logobj.is_some() {
            return logobj;
        }

        None
    }

    pub fn parse_killed_line(&self, line: &str) -> Option<LogLine> {
        let caps = self.killed_rx.captures(line);

        match caps {
            Some(caps) => {
                let killer = caps.get(1).unwrap().as_str().to_string();
                let victim = caps.get(2).unwrap().as_str().to_string();
                let weapon = caps.get(3).unwrap().as_str().to_string();
                let crit = caps.get(4).unwrap().as_str().ends_with(". (crit)");

                Some(LogLine::Kill {
                    killer,
                    victim,
                    weapon,
                    crit,
                })
            }
            None => None,
        }
    }

    pub fn parse_suicided_line(&self, line: &str) -> Option<LogLine> {
        let caps = self.suicided_rx.captures(line);

        match caps {
            Some(caps) => {
                let name = caps.get(1).unwrap().as_str().to_string();
                Some(LogLine::Suicide { name })
            }
            None => None,
        }
    }

    pub fn parse_lobby_status_line(&self, line: &str) -> Option<LogLine> {
        if line == "Lobby created" {
            return Some(LogLine::LobbyCreated);
        }

        if line == "Lobby destroyed" {
            return Some(LogLine::LobbyDestroyed);
        }

        if let Some(_caps) = self.differing_lobby_rx.captures(line) {
            return Some(LogLine::LobbyCreated);
        }

        None
    }

    pub fn parse_chat_line(&self, line: &str) -> Option<LogLine> {
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
                    name,
                    message,
                    dead,
                    team,
                })
            }
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_kill_line() {
        let parser = LogLineParser::default();

        let line = "Player1 killed Player2 with iron_bomber. (crit)";
        let result = parser.parse_line(line).unwrap();
        assert_eq!(
            result,
            LogLine::Kill {
                killer: "Player1".to_string(),
                victim: "Player2".to_string(),
                weapon: "iron_bomber".to_string(),
                crit: true,
            }
        );

        let line = "Player1 killed Player2 with iron_bomber.";
        let result = parser.parse_line(line).unwrap();
        assert_eq!(
            result,
            LogLine::Kill {
                killer: "Player1".to_string(),
                victim: "Player2".to_string(),
                weapon: "iron_bomber".to_string(),
                crit: false,
            }
        );

        let line = "1Pleaseburger killed used to facetime with your mom with obj_sentrygun3.";
        let result = parser.parse_line(line).unwrap();
        assert_eq!(
            result,
            LogLine::Kill {
                killer: "1Pleaseburger".to_string(),
                victim: "used to facetime with your mom".to_string(),
                weapon: "obj_sentrygun3".to_string(),
                crit: false,
            }
        );

        let line = "used to facetime with your mom killed used to facetime with your mom with obj_sentrygun3.";
        let result = parser.parse_line(line).unwrap();
        assert_eq!(
            result,
            LogLine::Kill {
                killer: "used to facetime with your mom".to_string(),
                victim: "used to facetime with your mom".to_string(),
                weapon: "obj_sentrygun3".to_string(),
                crit: false,
            }
        );
    }

    #[test]
    fn test_parse_suicided_line() {
        let parser = LogLineParser::default();

        let line = "Player1 suicided.";
        let result = parser.parse_line(line).unwrap();
        assert_eq!(
            result,
            LogLine::Suicide {
                name: "Player1".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_lobby_status_line() {
        let parser = LogLineParser::default();

        let line = r#"Lobby created"#;
        let result = parser.parse_line(line).unwrap();
        assert_eq!(result, LogLine::LobbyCreated);

        let line = r#"Lobby destroyed"#;
        let result = parser.parse_line(line).unwrap();
        assert_eq!(result, LogLine::LobbyDestroyed);
    }

    #[test]
    fn test_parse_chat_line() {
        let parser = LogLineParser::default();

        let line = "Player1 :  hello";
        let result = parser.parse_line(line).unwrap();
        assert_eq!(
            result,
            LogLine::Chat {
                name: "Player1".to_string(),
                message: "hello".to_string(),
                dead: false,
                team: false,
            }
        );

        let line = "(TEAM) Player1 :  hello again";
        let result = parser.parse_line(line).unwrap();
        assert_eq!(
            result,
            LogLine::Chat {
                name: "Player1".to_string(),
                message: "hello again".to_string(),
                dead: false,
                team: true,
            }
        );

        let line = "*DEAD* Player1 :  hello dead";
        let result = parser.parse_line(line).unwrap();
        assert_eq!(
            result,
            LogLine::Chat {
                name: "Player1".to_string(),
                message: "hello dead".to_string(),
                dead: true,
                team: false,
            }
        );

        let line = "*DEAD*(TEAM) Player1 :  hello again dead";
        let result = parser.parse_line(line).unwrap();
        assert_eq!(
            result,
            LogLine::Chat {
                name: "Player1".to_string(),
                message: "hello again dead".to_string(),
                dead: true,
                team: true,
            }
        );
    }

    #[test]
    fn test_lines_with_timestamp_are_stripped() {
        let parser = LogLineParser::default();

        let line = "05/08/2024 - 13:30:42: Player1 suicided.";
        let result = parser.parse_line(line).unwrap();
        assert_eq!(
            result,
            LogLine::Suicide {
                name: "Player1".to_string()
            }
        );
    }

    #[test]
    fn test_parse_order() {
        let parser = LogLineParser::default();

        let line = r#"Player1 suicided."#;
        let result = parser.parse_line(line).unwrap();
        assert_eq!(
            result,
            LogLine::Suicide {
                name: "Player1".to_string()
            }
        );

        let line = r#"Player1 :  suicided."#;
        let result = parser.parse_line(line).unwrap();
        assert_eq!(
            result,
            LogLine::Chat {
                name: "Player1".to_string(),
                message: "suicided.".to_string(),
                dead: false,
                team: false
            }
        );
    }

    // Differing lobby received. Lobby: [A:1:3982432273:50775]/Match117694416/Lobby713780413559521 CurrentlyAssigned: [A:1:3982432273:50775]/Match117693976/Lobby713780413559521
    #[test]
    fn test_new_lobby() {
        let parser = LogLineParser::default();

        let line = "Differing lobby received. Lobby: [A:1:3982432273:50775]/Match117694416/Lobby713780413559521 CurrentlyAssigned: [A:1:3982432273:50775]/Match117693976/Lobby713780413559521";
        let result = parser.parse_line(line).unwrap();
        assert_eq!(result, LogLine::LobbyCreated);
    }
}
