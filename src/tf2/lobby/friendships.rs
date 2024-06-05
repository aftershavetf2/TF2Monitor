use super::Lobby;
use crate::models::steamid::SteamID;
use std::collections::{HashMap, HashSet};

#[derive(Default, Debug, Clone)]
pub struct Friendships {
    friendships: HashMap<SteamID, HashSet<SteamID>>,
    empty_friendlist: HashSet<SteamID>,
}

impl Friendships {
    /// Create a mapping of steamid -> friends using the lobby data.
    /// For each player in the lobby, only include friends that are also in the lobby.
    /// To circumvent private profiles, also create the reverse mapping, each friend has the player as a friend.
    pub fn from_lobby(lobby: &Lobby) -> Self {
        // Only players in the lobby are of interest
        let lobby_steamids: HashSet<SteamID> = lobby.players.iter().map(|p| p.steamid).collect();

        let mut friendships: HashMap<SteamID, HashSet<SteamID>> = HashMap::new();

        for player in &lobby.players {
            let friends = player
                .friends
                .clone()
                .unwrap_or_default()
                .intersection(&lobby_steamids)
                .cloned()
                .collect();

            // Also create the reverse mapping, each friend has the player as a friend
            for friend in &friends {
                if let Some(friends) = friendships.get_mut(friend) {
                    friends.insert(player.steamid);
                } else {
                    let mut friends = HashSet::new();
                    friends.insert(player.steamid);
                    friendships.insert(*friend, friends);
                }
            }

            friendships.insert(player.steamid, friends);
        }

        Friendships {
            friendships,
            empty_friendlist: HashSet::new(),
        }
    }

    /// Get the friends of a player
    pub fn get_friends(&self, steamid: SteamID) -> &HashSet<SteamID> {
        if let Some(friends) = self.friendships.get(&steamid) {
            friends
        } else {
            &self.empty_friendlist
        }
    }

    pub fn are_friends(&self, steamid1: SteamID, steamid2: SteamID) -> bool {
        self.get_friends(steamid1).contains(&steamid2)
    }
}
