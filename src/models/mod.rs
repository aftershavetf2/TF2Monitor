pub mod app_settings;
pub mod steamid;

use self::{app_settings::AppSettings, steamid::SteamID};
use crate::{
    appbus::{AppBus, AppEventMsg},
    tf2::lobby::Lobby,
};
use bus::BusReader;
use eframe::egui::Pos2;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

pub struct AppWin {
    pub bus: Arc<Mutex<AppBus>>,

    pub app_settings: AppSettings,

    pub lobby: Lobby,
    pub lobby_report_bus_rx: BusReader<Lobby>,

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
            lobby_report_bus_rx: bus.lock().unwrap().lobby_report_bus.add_rx(),
            self_steamid: settings.self_steamid64,
            selected_player: None,
            spectating: false,

            friendship_positions: HashMap::new(),
        }
    }

    pub fn updated_settings(&mut self) {
        log::info!("Saving updated settings");
        self.app_settings.save();

        log::info!("Broadcasting updated settings");
        self.bus
            .lock()
            .unwrap()
            .app_event_bus
            .broadcast(AppEventMsg::UpdatedSettings(self.app_settings.clone()));
    }

    pub fn process_bus(&mut self) {
        log::debug!("Processing bus messages");
        while let Ok(lobby) = self.lobby_report_bus_rx.try_recv() {
            self.lobby = lobby;
        }
    }

    /// The selected player is who is shown in the player details panel.
    /// Toggle selected player if same, otherwise select new player
    pub fn set_selected_player(&mut self, clicked_on_steamid: SteamID) {
        if let Some(steamid) = self.selected_player {
            if steamid == clicked_on_steamid {
                self.selected_player = None;
            } else {
                self.selected_player = Some(clicked_on_steamid);
            }
        } else {
            self.selected_player = Some(clicked_on_steamid);
        }
    }
}
