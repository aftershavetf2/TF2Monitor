use super::Lobby;
use crate::models::steamid::SteamID;
use std::sync::{Arc, Mutex};

/// Shared lobby state that can be accessed from multiple threads.
/// Provides thread-safe access to the lobby with a convenient API.
///
/// The lobby is protected by a Mutex, but the lock is only held briefly
/// during get() and set() operations, not during long-running work.
pub struct SharedLobby {
    lobby: Arc<Mutex<Lobby>>,
}

impl SharedLobby {
    /// Create a new SharedLobby with an initial lobby state
    pub fn new(lobby: Lobby) -> Self {
        Self {
            lobby: Arc::new(Mutex::new(lobby)),
        }
    }

    /// Get a copy of the current lobby state.
    /// This is the recommended way to read lobby data from other threads.
    /// The copy is taken while holding the lock briefly, ensuring consistency.
    pub fn get(&self) -> Lobby {
        self.lobby.lock().unwrap().clone()
    }

    /// Set the lobby state.
    /// Use this when you have modified a copy of the lobby and want to update
    /// the shared state. The lock is only held briefly during the update.
    pub fn set(&self, lobby: Lobby) {
        *self.lobby.lock().unwrap() = lobby;
    }

    /// Update a specific player in the lobby.
    /// This is a convenience method for threads that need to update player data.
    pub fn update_player<F>(&self, steamid: SteamID, updater: F)
    where
        F: FnOnce(&mut crate::tf2::lobby::Player),
    {
        let mut lobby = self.get();
        if let Some(player) = lobby.get_player_mut(None, Some(steamid)) {
            updater(player);
            self.set(lobby);
        }
    }
}

impl Clone for SharedLobby {
    fn clone(&self) -> Self {
        Self {
            lobby: Arc::clone(&self.lobby),
        }
    }
}
