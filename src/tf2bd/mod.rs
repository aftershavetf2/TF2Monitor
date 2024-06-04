pub mod models;
pub mod ruleset_handler;
pub mod tf2bd_thread;

use crate::{
    appbus::AppBus,
    models::{app_settings::AppSettings, steamid::SteamID},
    tf2::lobby::PlayerMarking,
};
use std::sync::{Arc, Mutex};

/// Start the background threads for the TF2 module
pub fn start(settings: &AppSettings, bus: &Arc<Mutex<AppBus>>) {
    let tf2bd_thread_handle = tf2bd_thread::start(settings, bus);

    let mut bus = bus.lock().unwrap();
    bus.tf2bd_thread_handle = Some(tf2bd_thread_handle);
}

#[derive(Debug, Clone)]
pub enum Tf2bdMsg {
    Tf2bdPlayerMarking(SteamID, PlayerMarking),
}
