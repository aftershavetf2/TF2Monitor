mod appbus;
mod config;
mod db;
mod gui;
mod http_cache;
mod models;
mod reputation;
mod tf2;
mod tf2bd;
mod utils;

use appbus::AppBus;
use eframe::Result;
use models::app_settings::AppSettings;
use reputation::reputation_thread;
use std::sync::{Arc, Mutex};

fn main() -> Result<(), eframe::Error> {
    simple_logger::SimpleLogger::new().init().unwrap();

    log::info!("TF2Monitor is starting...");

    // Connect to database
    log::info!("Connecting to database...");
    let db = crate::db::db::connect().expect("Failed to connect to database");
    log::info!("Database connection established");

    let settings = AppSettings::load_or_default();
    let bus = Arc::new(Mutex::new(AppBus::new(settings.self_steamid64)));

    tf2::start(&settings, &bus, &db);
    tf2bd::tf2bd_thread::start(&settings, &bus, &db);
    reputation_thread::start(&settings, &bus, &db);

    gui::run(&settings, &bus)
}
