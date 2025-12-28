pub mod app_settings;
pub mod steamid;

use self::{app_settings::AppSettings, steamid::SteamID};
use crate::{
    appbus::{AppBus, AppEventMsg},
    tf2::lobby::{Lobby, shared_lobby::SharedLobby},
};
use eframe::egui::Pos2;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

pub struct AppWin {
    pub bus: Arc<Mutex<AppBus>>,

    pub app_settings: AppSettings,

    pub lobby: Lobby,
    pub shared_lobby: SharedLobby,

    pub self_steamid: SteamID,
    pub selected_player: Option<SteamID>,
    pub spectating: bool,

    // When drawing the scoreboard, we remember the center positions of each player's team indicator.
    // This is used to draw friendship indicators between players.
    pub friendship_positions: HashMap<SteamID, Pos2>,
}

impl AppWin {
    pub fn new(settings: &AppSettings, bus: &Arc<Mutex<AppBus>>) -> Self {
        Self {
            bus: Arc::clone(bus),

            app_settings: settings.clone(),

            lobby: Lobby::new(settings.self_steamid64),
            shared_lobby: bus.lock().unwrap().shared_lobby.clone(),
            self_steamid: settings.self_steamid64,
            selected_player: None,
            spectating: false,

            friendship_positions: HashMap::new(),
        }
    }

    pub fn updated_settings(&mut self) {
        log::info!("Saving and broadcasting updated settings");
        self.app_settings.save();
        self.bus
            .lock()
            .unwrap()
            .app_event_bus
            .broadcast(AppEventMsg::UpdatedSettings(self.app_settings.clone()));
    }

    pub fn get_latest_lobby(&mut self) {
        // Get a copy of the current lobby state
        self.lobby = self.shared_lobby.get();
    }

    /// The selected player is who is shown in the player details panel.
    pub fn set_selected_player(&mut self, clicked_on_steamid: SteamID) {
        self.selected_player = Some(clicked_on_steamid);
    }

    pub fn is_me(&self, steamid: Option<SteamID>) -> bool {
        match steamid {
            Some(steamid) => steamid == self.self_steamid,
            None => false,
        }
    }
}
