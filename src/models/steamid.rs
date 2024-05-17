use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SteamID(u64);

impl SteamID {
    pub fn from_u64(steamid64: u64) -> Self {
        Self(steamid64)
    }

    pub fn from_u64_string(steamid64: &String) -> Option<Self> {
        if let Ok(steamid64) = steamid64.parse::<u64>() {
            Some(Self::from_u64(steamid64))
        } else {
            log::debug!("Failed to parse SteamID64: '{}'", steamid64);
            None
        }
    }

    pub fn from_steam_id32(steamid32: &str) -> Self {
        let steamid32 = steamid32
            .trim_start_matches("[U:1:")
            .trim_end_matches(']')
            .parse::<u64>()
            .unwrap();

        Self::from_u64(steamid32 + 76561197960265728)
    }

    pub fn to_u64(self) -> u64 {
        self.0
    }

    pub fn is_valid(self) -> bool {
        self.0 >= 76561197960265728
    }

    pub fn steam_history_url(&self) -> String {
        format!("https://steamhistory.net/id/{}", self.0)
    }

    pub fn steam_community_url(&self) -> String {
        format!("https://steamcommunity.com/profiles/{}", self.0)
    }

    pub fn steam_rep_url(&self) -> String {
        format!("https://steamrep.com/search?q={}", self.0)
    }

    pub fn steam_id_url(&self) -> String {
        format!("https://steamid.uk/profile/{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_steamdid32() {
        assert_eq!(
            SteamID::from_steam_id32("[U:1:169802]"),
            SteamID::from_u64(76561197960435530)
        );
        assert_eq!(
            SteamID::from_steam_id32("[U:1:34093805]"),
            SteamID::from_u64(76561197994359533)
        );
        assert_eq!(
            SteamID::from_steam_id32("[U:1:1218982957]"),
            SteamID::from_u64(76561199179248685)
        );
    }
}
