#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

mod appbus;
mod gui;
mod models;
mod tf2;
mod tf2bd;
mod utils;

use appbus::AppBus;
use eframe::{egui, Result};
use log::{info, trace, warn};
use models::app_settings::AppSettings;
use std::sync::atomic::{AtomicBool, Ordering};
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

// fn main() {
//     simple_logger::SimpleLogger::new().init().unwrap();

//     let settings = AppSettings::load_or_default();
//     let buses = Arc::new(Mutex::new(AppBus::default()));

//     // test_steam_api(&settings);
//     test_get_friendslist(&settings);
// }

// fn test_steam_api(settings: &AppSettings) {
//     use crate::models::steamid::SteamID;

//     let mut steam_api = tf2::steam::SteamApi::new(settings);

//     // 76561199119901587,76561199289898291,76561199234573637,76561199191957545,76561199243399574,76561198064076891,76561199197733316,76561199174058886,76561198076719730,76561198884330277,76561199156370643,76561198090231678,76561198899495757,76561198147668557,76561198370670319,76561199467708430,76561198080281312,76561199379427015,76561198451145010,76561199400218364,76561199559924950,76561198999562072,76561199664152002,76561197974228301
//     let steamid64 = models::steamid::SteamID::from_steam_id32("[U:1:169802]");
//     let players = steam_api.get_player_summaries(vec![
//         SteamID::from_u64(76561199119901587),
//         SteamID::from_u64(76561199289898291),
//         SteamID::from_u64(76561199234573637),
//         SteamID::from_u64(76561199191957545),
//         SteamID::from_u64(76561199243399574),
//         SteamID::from_u64(76561198064076891),
//         SteamID::from_u64(76561199197733316),
//         SteamID::from_u64(76561199174058886),
//         SteamID::from_u64(76561198076719730),
//         SteamID::from_u64(76561198884330277),
//         SteamID::from_u64(76561199156370643),
//         SteamID::from_u64(76561198090231678),
//         SteamID::from_u64(76561198899495757),
//         SteamID::from_u64(76561198147668557),
//         SteamID::from_u64(76561198370670319),
//         SteamID::from_u64(76561199467708430),
//         SteamID::from_u64(76561198080281312),
//         SteamID::from_u64(76561199379427015),
//         SteamID::from_u64(76561198451145010),
//         SteamID::from_u64(76561199400218364),
//         SteamID::from_u64(76561199559924950),
//         SteamID::from_u64(76561198999562072),
//         SteamID::from_u64(76561199664152002),
//         SteamID::from_u64(76561197974228301),
//     ]);
//     println!("Players: {:?}", players);
// }

// fn test_get_friendslist(settings: &AppSettings) {
//     use crate::models::steamid::SteamID;

//     let mut steam_api = tf2::steam::SteamApi::new(settings);

//     let friends = steam_api.get_friendlist(settings.self_steamid64).unwrap();
//     println!("Friends: {:?}", friends);
//     println!("Friends.len(): {:?}", friends.len());
// }

fn main2() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Multiple viewports",
        options,
        Box::new(|_cc| Ok(Box::<MyApp>::default())),
    )
}

#[derive(Default)]
struct MyApp {
    /// Immediate viewports are show immediately, so passing state to/from them is easy.
    /// The downside is that their painting is linked with the parent viewport:
    /// if either needs repainting, they are both repainted.
    show_immediate_viewport: bool,

    /// Deferred viewports run independent of the parent viewport, which can save
    /// CPU if only some of the viewports require repainting.
    /// However, this requires passing state with `Arc` and locks.
    show_deferred_viewport: Arc<AtomicBool>,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Hello from the root viewport");

            ui.checkbox(
                &mut self.show_immediate_viewport,
                "Show immediate child viewport",
            );

            let mut show_deferred_viewport = self.show_deferred_viewport.load(Ordering::Relaxed);
            ui.checkbox(&mut show_deferred_viewport, "Show deferred child viewport");
            self.show_deferred_viewport
                .store(show_deferred_viewport, Ordering::Relaxed);
        });

        if self.show_immediate_viewport {
            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("immediate_viewport"),
                egui::ViewportBuilder::default()
                    .with_title("Immediate Viewport")
                    .with_inner_size([200.0, 100.0]),
                |ctx, class| {
                    assert!(
                        class == egui::ViewportClass::Immediate,
                        "This egui backend doesn't support multiple viewports"
                    );

                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.label("Hello from immediate viewport");
                    });

                    if ctx.input(|i| i.viewport().close_requested()) {
                        // Tell parent viewport that we should not show next frame:
                        self.show_immediate_viewport = false;
                    }
                },
            );
        }

        if self.show_deferred_viewport.load(Ordering::Relaxed) {
            let show_deferred_viewport = self.show_deferred_viewport.clone();
            ctx.show_viewport_deferred(
                egui::ViewportId::from_hash_of("deferred_viewport"),
                egui::ViewportBuilder::default()
                    .with_title("Deferred Viewport")
                    .with_inner_size([200.0, 100.0]),
                move |ctx, class| {
                    assert!(
                        class == egui::ViewportClass::Deferred,
                        "This egui backend doesn't support multiple viewports"
                    );

                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.label("Hello from deferred viewport");
                    });
                    if ctx.input(|i| i.viewport().close_requested()) {
                        // Tell parent to close us.
                        show_deferred_viewport.store(false, Ordering::Relaxed);
                    }
                },
            );
        }
    }
}
