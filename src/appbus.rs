use bus::Bus;

use crate::{
    models::steamid::SteamID,
    tf2::{
        lobby::{Lobby, PlayerFlag},
        logfile::LogLine,
        steamapi::SteamApiMsg,
    },
    tf2bd::Tf2bdMsg,
};

pub struct AppBus {
    pub logfile_bus: Bus<LogLine>,
    pub rcon_bus: Bus<String>,
    pub lobby_report_bus: Bus<Lobby>,
    pub steamapi_bus: Bus<SteamApiMsg>,
    pub tf2bd_bus: Bus<Tf2bdMsg>,

    /// The events mostly from the user interface.
    /// Many different parts of the application can listen to these events.
    pub app_event_bus: Bus<AppEventMsg>,

    pub rcon_thread_handle: Option<std::thread::JoinHandle<()>>,
    pub lobby_thread_handle: Option<std::thread::JoinHandle<()>>,
    pub steamapi_thread_handle: Option<std::thread::JoinHandle<()>>,
    pub logfile_watcher_thread_handle: Option<std::thread::JoinHandle<()>>,
    pub tf2bd_thread_handle: Option<std::thread::JoinHandle<()>>,
}

impl Default for AppBus {
    fn default() -> Self {
        Self::new()
    }
}

impl AppBus {
    pub fn new() -> Self {
        Self {
            logfile_bus: Bus::new(10000),
            rcon_bus: Bus::new(10),
            lobby_report_bus: Bus::new(10),
            steamapi_bus: Bus::new(1000),
            tf2bd_bus: Bus::new(1000),
            app_event_bus: Bus::new(100),

            rcon_thread_handle: None,
            lobby_thread_handle: None,
            steamapi_thread_handle: None,
            logfile_watcher_thread_handle: None,
            tf2bd_thread_handle: None,
        }
    }

    pub fn send_logline(&mut self, logline: LogLine) {
        self.logfile_bus.broadcast(logline);
    }

    pub fn send_lobby_report(&mut self, lobby: Lobby) {
        self.lobby_report_bus.broadcast(lobby);
    }

    /// Send a RCON command to the TF2 RCON
    #[allow(dead_code)]
    pub fn send_rcon_cmd(&mut self, cmd: &str) {
        log::info!("Sending RCON command: {}", cmd);
        self.rcon_bus.broadcast(cmd.to_string());
    }

    // pub fn health_report(&self) {
    //     log::info!("Health report");
    //     log::info!(
    //         "rcon_thread_handle: is_finished {:?}",
    //         self.rcon_thread_handle.as_ref().unwrap().is_finished()
    //     );
    //     log::info!(
    //         "lobby_thread_handle: is_finished {:?}",
    //         self.lobby_thread_handle.as_ref().unwrap().is_finished()
    //     );
    //     log::info!(
    //         "logfile_watcher_thread_handle: is_finished {:?}",
    //         self.logfile_watcher_thread_handle
    //             .as_ref()
    //             .unwrap()
    //             .is_finished()
    //     );
    // }
}

#[derive(Debug, Clone)]
pub enum AppEventMsg {
    /// Sets or removes a flag(Cheater, Exploiter, etc) for a SteamID
    SetPlayerFlag(SteamID, PlayerFlag, bool),
}
