use crate::{
    models::steamid::SteamID,
    models::{
        app_settings::AppSettings,
        player_markings::{MarkingCommand, PlayerMarkings},
    },
    tf2::{
        lobby::{Lobby, shared_lobby::SharedLobby},
        logfile::LogLine,
        rcon::G15DumpPlayerOutput,
        steamapi::SteamApiMsg,
    },
    tf2bd::{Tf2bdMsg, player_lists::SharedMatches},
};
use bus::Bus;

pub struct AppBus {
    pub markings: std::sync::Arc<PlayerMarkings>,
    pub marking_commands: Option<std::sync::mpsc::Sender<MarkingCommand>>,
    pub profile_events: Bus<crate::models::player_profile::ProfileEvent>,
    pub profile_requests: std::collections::HashSet<SteamID>,
    pub ban_data: crate::ban_data::Control,
    pub logfile_bus: Bus<LogLine>,
    pub rcon_bus: Bus<String>,
    pub g15_report_bus: Bus<G15DumpPlayerOutput>,
    pub steamapi_bus: Bus<SteamApiMsg>,
    pub tf2bd_bus: Bus<Tf2bdMsg>,
    pub tf2bd_matches: SharedMatches,

    /// The events mostly from the user interface.
    /// Many different parts of the application can listen to these events.
    pub app_event_bus: Bus<AppEventMsg>,

    /// Shared lobby state accessible from all threads.
    /// Use shared_lobby.get() to get a copy of the current lobby state.
    pub shared_lobby: SharedLobby,
}

impl Default for AppBus {
    fn default() -> Self {
        // Use a default SteamID for default initialization
        // In practice, AppBus::new() should be called with the actual SteamID
        Self::new(crate::models::steamid::SteamID::from_u64(0))
    }
}

impl AppBus {
    pub fn new(self_steamid: crate::models::steamid::SteamID) -> Self {
        let initial_lobby = Lobby::new(self_steamid);
        Self {
            markings: Default::default(),
            marking_commands: None,
            profile_events: Bus::new(10000),
            profile_requests: Default::default(),
            ban_data: crate::ban_data::Control::default(),
            logfile_bus: Bus::new(10000),
            rcon_bus: Bus::new(100),
            g15_report_bus: Bus::new(100),
            steamapi_bus: Bus::new(10000),
            tf2bd_bus: Bus::new(10000),
            tf2bd_matches: SharedMatches::default(),
            app_event_bus: Bus::new(1000),
            shared_lobby: SharedLobby::new(initial_lobby),
        }
    }

    pub fn send_logline(&mut self, logline: LogLine) {
        self.logfile_bus.broadcast(logline);
    }

    /// Send a RCON command to the TF2 RCON
    pub fn send_rcon_cmd(&mut self, cmd: &str) {
        log::info!("Sending RCON command: {}", cmd);
        self.rcon_bus.broadcast(cmd.to_string());
    }
}

#[derive(Debug, Clone)]
pub enum AppEventMsg {
    UpdatedSettings(AppSettings),
}
