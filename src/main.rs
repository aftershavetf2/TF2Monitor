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
use std::{
    sync::{Arc, Mutex},
    thread,
};
use tokio::runtime::Runtime;

/// Setup tokio runtime in a separate thread for async operations in background threads.
/// This runtime can be used by background threads that need to run async code (e.g., database operations).
/// The runtime is kept alive for the lifetime of the application.
fn setup_async_runtime() -> tokio::runtime::Handle {
    let rt = Runtime::new().expect("Failed to create tokio runtime");
    let handle = rt.handle().clone();

    // Keep runtime alive in a background thread
    thread::spawn(move || {
        rt.block_on(async {
            // Keep the runtime alive
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
            }
        });
    });

    handle
}

fn main() -> Result<(), eframe::Error> {
    simple_logger::SimpleLogger::new().init().unwrap();

    log::info!("TF2Monitor is starting...");

    // Setup async runtime for background threads that need async support (e.g., database operations)
    let _async_handle = setup_async_runtime();

    let settings = AppSettings::load_or_default();
    let bus = Arc::new(Mutex::new(AppBus::default()));

    tf2::start(&settings, &bus);
    tf2bd::tf2bd_thread::start(&settings, &bus);
    reputation_thread::start(&settings, &bus);

    gui::run(&settings, &bus)
}
