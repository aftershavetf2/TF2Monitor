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
            let mut friends: HashSet<SteamID> = player
                .friends
                .clone()
                .unwrap_or_default()
                .intersection(&lobby_steamids)
                .cloned()
                .collect();

            if let Some(existing_friends) = friendships.get(&player.steamid) {
                friends.extend(existing_friends);
            }

            // Also create the reverse mapping, each friend has this player as a friend
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

#[cfg(test)]
mod tests {
    use crate::{models::steamid::MIN_STEAMID64, tf2::lobby::Player};

    use super::*;

    /// Test if friendships are correctly created from a lobby.
    /// Friendships should be bidirectional.
    /// So even if a player has a private profile,
    /// if another player has them as a friend, they should be friends as well.
    #[test]
    fn test_bidirectional_friendship() {
        let player1 = SteamID::from_u64(MIN_STEAMID64);
        let player2 = SteamID::from_u64(MIN_STEAMID64 + 1);
        let player3 = SteamID::from_u64(MIN_STEAMID64 + 2);

        let mut lobby = Lobby {
            players: vec![
                // Player1: A player with public profile
                Player {
                    steamid: player1,
                    friends: Some(vec![player2, player3].into_iter().collect()),
                    ..Default::default()
                },
                // Player2: A player with private profile
                Player {
                    steamid: player2,
                    friends: None,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        lobby.friendships = Friendships::from_lobby(&lobby);
        println!("{:?}", lobby.friendships.friendships);

        assert!(lobby.friendships.are_friends(player1, player2));
        assert!(lobby.friendships.are_friends(player2, player1));
    }
}
