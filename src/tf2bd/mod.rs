pub mod models;
pub mod ruleset_handler;
pub mod tf2bd_thread;

use crate::{appbus::AppBus, models::app_settings::AppSettings};
use std::sync::{Arc, Mutex};

/// Start the background threads for the TF2 module
pub fn start(settings: &AppSettings, bus: &Arc<Mutex<AppBus>>) {
    let tf2bd_thread_handle = tf2bd_thread::start(settings, bus);

    let mut bus = bus.lock().unwrap();
    bus.tf2bd_thread_handle = Some(tf2bd_thread_handle);
}

pub struct Tf2dMsg {}
