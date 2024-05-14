use std::sync::{Arc, Mutex};

use appbus::AppBus;
use models::app_settings::AppSettings;

mod appbus;
mod gui;
mod models;
mod tf2;
mod utils;

fn main() -> Result<(), eframe::Error> {
    simple_logger::SimpleLogger::new().init().unwrap();

    let settings = AppSettings::load_or_default();
    let buses = Arc::new(Mutex::new(AppBus::default()));

    tf2::start(&settings, &buses);

    gui::run(&settings, &buses)
}
