pub mod app_settings;
pub mod steamid;

use self::{app_settings::AppSettings, steamid::SteamID};
use crate::{appbus::AppBus, tf2::lobby::Lobby};
use bus::BusReader;
use eframe::egui::Pos2;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

pub struct AppWin {
    pub bus: Arc<Mutex<AppBus>>,

    pub lobby: Lobby,
    pub lobby_report_bus_rx: BusReader<Lobby>,

    pub self_steamid: SteamID,
    pub swap_team_colors: bool,
    pub show_crits: bool,
    pub selected_player: Option<SteamID>,

    pub show_friendships: bool,

    // When drawing the scoreboard, we remember the center positions of eacg player's team indicator.
    // This is used to draw friendship indicators between players.
    // This should be a HashMap but f32 doesn't implement Hash.
    pub friendship_positions: Vec<(SteamID, Pos2)>,
    pub friendship_positions2: HashMap<SteamID, Pos2>,
}

impl AppWin {
    pub fn new(settings: &AppSettings, bus: &Arc<Mutex<AppBus>>) -> Self {
        Self {
            bus: Arc::clone(bus),

            lobby: Lobby::new(),
            lobby_report_bus_rx: bus.lock().unwrap().lobby_report_bus.add_rx(),
            swap_team_colors: false,
            show_crits: true,
            show_friendships: true,
            self_steamid: settings.self_steamid64,
            selected_player: None,

            friendship_positions: Vec::new(),
            friendship_positions2: HashMap::new(),
        }
    }

    pub fn process_bus(&mut self) {
        while let Ok(lobby) = self.lobby_report_bus_rx.try_recv() {
            self.lobby = lobby;
        }
    }
}
