mod appbus;
mod db;
mod gui;
mod models;
mod tf2;
mod tf2bd;
mod utils;

use appbus::AppBus;
use eframe::Result;
use models::app_settings::AppSettings;
use std::sync::{Arc, Mutex};

fn main() -> Result<(), eframe::Error> {
    simple_logger::SimpleLogger::new().init().unwrap();

    log::info!("TF2Monitor is starting...");

    let settings = AppSettings::load_or_default();
    let bus = Arc::new(Mutex::new(AppBus::default()));

    tf2::start(&settings, &bus);
    tf2bd::tf2bd_thread::start(&settings, &bus);

    gui::run(&settings, &bus)
}
