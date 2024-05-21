use super::{Lobby, Player};
use crate::tf2::steam::SteamApi;

pub fn add_friends_from_steam(steam_api: &SteamApi, lobby: &mut Lobby) {
    // To fetch additional info from Steam Web Api a key is needed
    if !steam_api.has_key() {
        return;
    }

    // The fetching is synchronous, so we limit the amount of fetches
    // to allow for other info to be processed in between
    const MAX_FETCHES: usize = 3;

    // The list of players that
    // - Has basic info from Steam fetched
    // - But has no friends list fetched yet
    let players: Vec<&mut Player> = lobby
        .players
        .iter_mut()
        .filter(|p| {
            if let Some(steam_info) = &p.steam_info {
                return steam_info.friends.is_none();
            }

            false
        })
        .take(MAX_FETCHES)
        .collect();

    for player in players {
        if let Some(steam_info) = &mut player.steam_info {
            log::info!("Fetching friends of {}", player.name);

            if let Some(friends) = steam_api.get_friendlist(steam_info.steamid) {
                steam_info.friends = Some(friends);
            } else {
                steam_info.friends = Some(Vec::new());
            }
        }
    }
}
