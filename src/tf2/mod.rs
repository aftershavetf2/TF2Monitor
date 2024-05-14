use crate::{appbus::AppBus, models::app_settings::AppSettings};
use std::sync::{Arc, Mutex};

pub mod lobby;
pub mod logfile;
pub mod rcon;

/// Start the background threads for the TF2 module
pub fn start(settings: &AppSettings, bus: &Arc<Mutex<AppBus>>) {
    let rcon_thread_handle = rcon::rcon_thread::start(settings, bus);
    let lobby_thread_handle = lobby::lobby_thread::start(settings, bus);

    let logfile_watcher_thread_handle = logfile::logfile_watcher::start(settings, bus);

    let mut bus = bus.lock().unwrap();
    bus.rcon_thread_handle = Some(rcon_thread_handle);
    bus.lobby_thread_handle = Some(lobby_thread_handle);
    bus.logfile_watcher_thread_handle = Some(logfile_watcher_thread_handle);
}
