mod appbus;
mod ban_data;
mod config;
mod db;
mod gui;
mod models;
mod reputation;
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

    // Connect to database
    log::info!("Connecting to database...");
    let db = crate::db::connection::connect().expect("Failed to connect to database");
    log::info!("Database connection established");

    let settings = AppSettings::load_or_default();
    let bus = Arc::new(Mutex::new(AppBus::new(settings.self_steamid64)));

    let (marking_commands, markings_worker) = models::player_markings::start(&db, &bus)
        .expect("Failed to load player-markings.json");

    tf2::start(&settings, &bus, &db);
    tf2bd::tf2bd_thread::start(&settings, &bus);
    if let Err(error) = ban_data::start(&bus) {
        log::error!("Could not start community ban data: {error}");
    }

    let result = gui::run(&settings, &bus, Arc::new(db));
    let _ = marking_commands.send(models::player_markings::MarkingCommand::Stop);
    if markings_worker.join().is_err() {
        log::error!("Player markings worker stopped unexpectedly.");
    }
    result
}

// -novid +fps_max 300 -console -usercon -high +developer 1 +contimes 0 +ip 0.0.0.0 +net_start  +sv_rcon_whitelist_address 127.0.0.1 +rcon_password rconpwd +hostport 40434 +net_start +con_timestamp 1 -condebug -conclearlog -console -g15 -nojoy -nosteamcontroller -nohltv -particles 1
//
