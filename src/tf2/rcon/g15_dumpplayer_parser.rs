use std::collections::HashMap;

use regex::Regex;

use crate::{
    models::steamid::{self, SteamID},
    tf2::lobby::Team,
};

#[derive(Debug, Clone, Default)]

pub struct G15DumpPlayerOutput {
    pub players: Vec<G15PlayerData>,
}

#[derive(Debug, Clone, Default)]
pub struct G15PlayerData {
    pub steamid: SteamID,
    pub id: Option<i64>,
    pub name: Option<String>,
    pub ping: Option<i64>,
    pub alive: Option<bool>,
    pub team: Option<Team>,
    pub score: Option<i64>,
}

pub struct G15DumpPlayerParser {
    names_regex: Regex,
    pings_regex: Regex,
    accountids_regex: Regex,
    alives_regex: Regex,
    valids_regex: Regex,
    teams_regex: Regex,
    ids_regex: Regex,
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
            ids_regex: Regex::new(r"^m_iUserID\[(\d+)\] integer \((\d+)\)$").unwrap(),

            // m_iScore[10] integer (2688)
            scores_regex: Regex::new(r"^m_iScore\[(\d+)\] integer \((\d+)\)$").unwrap(),
        }
    }

    pub fn parse(&self, dump: &str) -> G15DumpPlayerOutput {
        let lines: Vec<&str> = dump
            .lines()
            .filter(|x| {
                x.starts_with("m_szName[")
                    || x.starts_with("m_iPing[")
                    || x.starts_with("m_iAccountID[")
                    || x.starts_with("m_bAlive[")
                    || x.starts_with("m_bValid[")
                    || x.starts_with("m_iTeam[")
                    || x.starts_with("m_iUserID[")
                    || x.starts_with("m_iScore[")
            })
            .collect::<Vec<&str>>();

        let names = self.get_names(&lines);
        let accountids = self.get_accountids(&lines);
        let pings = self.get_pings(&lines);
        let alives = self.get_alives(&lines);
        let valids = self.get_valids(&lines);
        let teams = self.get_teams(&lines);
        let ids = self.get_ids(&lines);
        let scores = self.get_scores(&lines);

        let mut result = G15DumpPlayerOutput::default();

        for i in 0u32..(valids.len() as u32) {
            if !valids.contains_key(&i)
                || !accountids.contains_key(&i)
                || !ids.contains_key(&i)
                || !names.contains_key(&i)
                || !pings.contains_key(&i)
                || !alives.contains_key(&i)
                || !teams.contains_key(&i)
                || !scores.contains_key(&i)
            {
                continue;
            }

            if !valids[&i] {
                continue;
            }

            if ids[&i] == 0 {
                continue;
            }

            let team = match teams.get(&i) {
                Some(2) => Some(Team::Red),
                Some(3) => Some(Team::Blue),
                _ => None,
            };

            let steamid =
                steamid::SteamID::from_u64(accountids[&i] as u64 + steamid::MIN_STEAMID64);

            let player = G15PlayerData {
                id: ids.get(&i).copied(),
                name: names.get(&i).cloned(),
                steamid,
                ping: pings.get(&i).copied(),
                alive: alives.get(&i).copied(),
                team,
                score: scores.get(&i).copied(),
            };
            result.players.push(player);
        }

        result
    }

    fn get_names(&self, lines: &Vec<&str>) -> HashMap<u32, String> {
        Self::get_strings(lines, &self.names_regex, "m_szName")
    }

    fn get_pings(&self, lines: &Vec<&str>) -> HashMap<u32, i64> {
        Self::get_i64s(
            lines,
            &self.pings_regex,
            "Failed to parse m_iPing value",
            "m_iPing",
        )
    }

    fn get_accountids(&self, lines: &Vec<&str>) -> HashMap<u32, i64> {
        Self::get_i64s(
            lines,
            &self.accountids_regex,
            "Failed to parse m_iAccountID value",
            "m_iAccountID",
        )
    }

    fn get_alives(&self, lines: &Vec<&str>) -> HashMap<u32, bool> {
        Self::get_bools(
            lines,
            &self.alives_regex,
            "Failed to parse m_bAlive value",
            "m_bAlive",
        )
    }

    fn get_valids(&self, lines: &Vec<&str>) -> HashMap<u32, bool> {
        Self::get_bools(
            lines,
            &self.valids_regex,
            "Failed to parse m_bValid value",
            "m_bValid",
        )
    }

    fn get_teams(&self, lines: &Vec<&str>) -> HashMap<u32, i64> {
        Self::get_i64s(
            lines,
            &self.teams_regex,
            "Failed to parse m_iTeam value",
            "m_iTeam",
        )
    }

    fn get_ids(&self, lines: &Vec<&str>) -> HashMap<u32, i64> {
        Self::get_i64s(
            lines,
            &self.ids_regex,
            "Failed to parse m_iUserID value",
            "m_iUserID",
        )
    }

    fn get_scores(&self, lines: &Vec<&str>) -> HashMap<u32, i64> {
        Self::get_i64s(
            lines,
            &self.scores_regex,
            "Failed to parse m_iScore value",
            "m_iScore",
        )
    }

    //
    // Helpers
    //

    fn get_strings(lines: &Vec<&str>, regex: &Regex, prefix: &str) -> HashMap<u32, String> {
        let mut result = HashMap::new();

        for line in lines {
            if line.starts_with(prefix) {
                if let Some(caps) = regex.captures(line) {
                    let id = caps.get(1).unwrap().as_str().parse::<u32>().unwrap();
                    let name = caps.get(2).unwrap().as_str().to_string();
                    result.insert(id, name);
                }
            }
        }

        result
    }

    fn get_i64s(
        lines: &Vec<&str>,
        regex: &Regex,
        expect_str: &str,
        prefix: &str,
    ) -> HashMap<u32, i64> {
        let mut result = HashMap::new();

        for line in lines {
            if line.starts_with(prefix) {
                if let Some(caps) = regex.captures(line) {
                    let id = caps.get(1).unwrap().as_str().parse::<u32>().unwrap();
                    let value = caps
                        .get(2)
                        .unwrap()
                        .as_str()
                        .parse::<i64>()
                        .expect(expect_str);
                    result.insert(id, value);
                }
            }
        }

        result
    }

    fn get_bools(
        lines: &Vec<&str>,
        regex: &Regex,
        expect_str: &str,
        prefix: &str,
    ) -> HashMap<u32, bool> {
        let mut result = HashMap::new();

        for line in lines {
            if line.starts_with(prefix) {
                if let Some(caps) = regex.captures(line) {
                    let id = caps.get(1).unwrap().as_str().parse::<u32>().unwrap();
                    let value = caps
                        .get(2)
                        .unwrap()
                        .as_str()
                        .parse::<bool>()
                        .expect(expect_str);
                    result.insert(id, value);
                }
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    // use chrono::prelude::*;

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
        assert_eq!("aftershave".to_string(), player.name.clone().unwrap());
        assert_eq!(SteamID::from_u64(76561197974228301), player.steamid);
        assert_eq!(23, output.players[15].ping.unwrap());
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

    #[test]
    fn test_get_names() {
        let parser = G15DumpPlayerParser::new();

        let dump = get_dump_text();
        let lines: Vec<&str> = dump.lines().collect::<Vec<&str>>();

        let output = parser.get_names(&lines);
        // for (i, name) in output.iter().enumerate() {
        //     println!("{}: {}", i, name);
        // }

        assert_eq!(output.len(), 102);

        /*
            From g15_dumpplayer_output.txt:
            m_szName[0] string ()
            m_szName[1] string (errrrrrrrrrrrrrrrrr)
            m_szName[2] string (szoboszlaiszilvia1979)
            m_szName[5] string (MechaFish)
            m_szName[3] string (Filipos3g)
            m_szName[6] string (Piggas in Naris)
            m_szName[4] string (mairo)
            m_szName[7] string (CubeCat0)
        */
        assert_eq!(output[&0], "");
        assert_eq!(output[&1], "errrrrrrrrrrrrrrrrr");
        assert_eq!(output[&2], "szoboszlaiszilvia1979");
        assert_eq!(output[&5], "MechaFish");
        assert_eq!(output[&3], "Filipos3g");
        assert_eq!(output[&6], "Piggas in Naris");
        assert_eq!(output[&4], "mairo");
        assert_eq!(output[&7], "CubeCat0");

        assert_eq!(output[&17], "aftershave");
    }
}
