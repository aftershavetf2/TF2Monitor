use regex::Regex;

use crate::{
    models::steamid::{self},
    tf2::{lobby::Team, rcon::G15PlayerData},
};

use super::G15DumpPlayerOutput;

pub struct G15DumpPlayerParser {
    names_regex: Regex,
    pings_regex: Regex,
    accountids_regex: Regex,
    alives_regex: Regex,
    valids_regex: Regex,
    teams_regex: Regex,
    userids_regex: Regex,
    scores_regex: Regex,
}

impl G15DumpPlayerParser {
    pub fn new() -> Self {
        Self {
            // m_szName[3] string (jawa)
            names_regex: Regex::new(r"^m_szName\[(\d+)\] string \((.*)\)$").unwrap(),

            // m_iPing[5] integer (56)
            pings_regex: Regex::new(r"^m_iPing\[(\d+)\] integer \((\d+)\)$").unwrap(),

            // m_iAccountID[1] integer (296910814)
            accountids_regex: Regex::new(r"^m_iAccountID\[(\d+)\] integer \((\d+)\)$").unwrap(),

            // m_bAlive[4] bool (true)
            alives_regex: Regex::new(r"^m_bAlive\[(\d+)\] bool \((true|false)\)$").unwrap(),

            // m_bValid[24] bool (true)
            valids_regex: Regex::new(r"^m_bValid\[(\d+)\] bool \((true|false)\)$").unwrap(),

            // m_iTeam[24] integer (2)
            teams_regex: Regex::new(r"^m_iTeam\[(\d+)\] integer \((\d+)\)$").unwrap(),

            // m_iUserID[10] integer (2688)
            userids_regex: Regex::new(r"^m_iUserID\[(\d+)\] integer \((\d+)\)$").unwrap(),

            // m_iScore[10] integer (2688)
            scores_regex: Regex::new(r"^m_iScore\[(\d+)\] integer \((\d+)\)$").unwrap(),
        }
    }

    pub fn parse(&self, dump: &str) -> G15DumpPlayerOutput {
        const N: usize = 102;

        let mut names: [&str; N] = [""; N];

        let mut valids: [bool; N] = [false; N];
        let mut alives: [bool; N] = [false; N];

        let mut pings: [i64; N] = [0; N];
        let mut accountids: [i64; N] = [0; N];
        let mut teams: [i64; N] = [0; N];
        let mut userids: [i64; N] = [0; N];
        let mut scores: [i64; N] = [0; N];

        for line in dump.lines() {
            // Strings
            if line.starts_with("m_szName[") {
                if let Some(caps) = self.names_regex.captures(line) {
                    let id = caps.get(1).unwrap().as_str().parse::<usize>().unwrap();
                    let value = caps.get(2).unwrap().as_str();
                    names[id] = value;
                }
            }
            // Bools
            if line.starts_with("m_bValid[") {
                if let Some(caps) = self.valids_regex.captures(line) {
                    let id = caps.get(1).unwrap().as_str().parse::<usize>().unwrap();
                    let value = caps
                        .get(2)
                        .unwrap()
                        .as_str()
                        .parse::<bool>()
                        .expect("Failed to parse m_bValid value");
                    valids[id] = value;
                }
            }
            if line.starts_with("m_bAlive[") {
                if let Some(caps) = self.alives_regex.captures(line) {
                    let id = caps.get(1).unwrap().as_str().parse::<usize>().unwrap();
                    let value = caps
                        .get(2)
                        .unwrap()
                        .as_str()
                        .parse::<bool>()
                        .expect("Failed to parse m_bAlive value");
                    alives[id] = value;
                }
            }
            // i64's
            if line.starts_with("m_iPing[") {
                if let Some(caps) = self.pings_regex.captures(line) {
                    let id = caps.get(1).unwrap().as_str().parse::<usize>().unwrap();
                    let value = caps
                        .get(2)
                        .unwrap()
                        .as_str()
                        .parse::<i64>()
                        .expect("Failed to parse m_iPing value");
                    pings[id] = value;
                }
            }
            if line.starts_with("m_iAccountID[") {
                if let Some(caps) = self.accountids_regex.captures(line) {
                    let id = caps.get(1).unwrap().as_str().parse::<usize>().unwrap();
                    let value = caps
                        .get(2)
                        .unwrap()
                        .as_str()
                        .parse::<i64>()
                        .expect("Failed to parse m_iAccountID value");
                    accountids[id] = value;
                }
            }
            if line.starts_with("m_iTeam[") {
                if let Some(caps) = self.teams_regex.captures(line) {
                    let id = caps.get(1).unwrap().as_str().parse::<usize>().unwrap();
                    let value = caps
                        .get(2)
                        .unwrap()
                        .as_str()
                        .parse::<i64>()
                        .expect("Failed to parse m_iTeam value");
                    teams[id] = value;
                }
            }
            if line.starts_with("m_iUserID[") {
                if let Some(caps) = self.userids_regex.captures(line) {
                    let id = caps.get(1).unwrap().as_str().parse::<usize>().unwrap();
                    let value = caps
                        .get(2)
                        .unwrap()
                        .as_str()
                        .parse::<i64>()
                        .expect("Failed to parse m_iUserID value");
                    userids[id] = value;
                }
            }
            if line.starts_with("m_iScore[") {
                if let Some(caps) = self.scores_regex.captures(line) {
                    let id = caps.get(1).unwrap().as_str().parse::<usize>().unwrap();
                    let value = caps
                        .get(2)
                        .unwrap()
                        .as_str()
                        .parse::<i64>()
                        .expect("Failed to parse m_iScore value");
                    scores[id] = value;
                }
            }
        }

        let mut result = G15DumpPlayerOutput::default();

        for i in 0usize..N {
            let valid = valids[i];
            let accountid = accountids[i] as u64;
            let userid = userids[i];

            if !valid || userid == 0 || accountid == 0 {
                continue;
            }

            let name = names[i];
            let ping = pings[i];
            let alive = alives[i];
            let team = teams[i];
            let score = scores[i];

            let steamid = steamid::SteamID::from_u64(accountid + steamid::MIN_STEAMID64);

            let player = G15PlayerData {
                id: userid,
                name: name.to_string(),
                steamid,
                ping,
                alive,
                team: match team {
                    2 => Some(Team::Red),
                    3 => Some(Team::Blue),
                    _ => None,
                },
                score,
            };
            result.players.push(player);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    // use chrono::prelude::*;

    use steamid::SteamID;

    use super::*;

    fn get_dump_text() -> String {
        let bytes = include_bytes!("g15_dumpplayer_output.txt");
        let s = String::from_utf8_lossy(bytes);
        s.to_string()
    }

    #[test]
    fn test_parse() {
        let parser = G15DumpPlayerParser::new();

        let dump = get_dump_text();
        let output = parser.parse(&dump);

        for (i, player) in output.players.iter().enumerate() {
            println!("{}: {:?}", i, player);
        }

        assert_eq!(19, output.players.len());
        let player = &output.players[15];
        assert_eq!("aftershave".to_string(), player.name);
        assert_eq!(SteamID::from_u64(76561197974228301), player.steamid);
        assert_eq!(23, output.players[15].ping);
    }

    // Without the filter in parse() this took 17 seconds.
    // With the filter in parse() it takes 10 seconds.
    // With a .start_with(prefix) it takes 5 seconds.
    // #[test]
    // fn test_bench_parse() {
    //     let parser = G15DumpPlayerParser::new();

    //     let dump = get_dump_text();

    //     let start_time = std::time::Instant::now();
    //     for _ in 0..1000 {
    //         let _output = parser.parse(&dump);
    //     }
    //     let stop_time = std::time::Instant::now();
    //     let elapsed = stop_time - start_time;
    //     println!("Elapsed: {:?}", elapsed);

    //     assert!(false);
    // }
}
