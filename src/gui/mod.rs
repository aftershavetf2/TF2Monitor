pub mod account_age;
pub mod background_image;
pub mod chat;
pub mod colors;
pub mod comments;
pub mod friendship_indicators;
pub mod kill_feed;
pub mod markings;
pub mod player_details_panel;
pub mod player_flag_editor;
pub mod player_tooltip;
pub mod playtime;
pub mod recently_left;
pub mod scoreboard;
pub mod scoreboard_team;
pub mod db_statistics_window;
pub mod settings_window;
pub mod top_menu;
pub mod ui_utils;
pub mod window_status_row;

use self::friendship_indicators::add_friendship_indicators;
use crate::{
    appbus::AppBus,
    config::{GUI_REPAINT_DELAY, GUI_SLEEP_DELAY},
    db::db::DbPool,
    models::{app_settings::AppSettings, AppWin},
};
use background_image::get_background_image_desc;
use chat::add_chat;
use eframe::egui::{self};
use kill_feed::add_kill_feed;
use db_statistics_window::show_db_statistics_window;
use player_details_panel::add_player_details_panel;
use settings_window::show_settings_window;
use std::{
    sync::{Arc, Mutex},
    thread::sleep,
};
use top_menu::add_top_menu;
use window_status_row::add_status_row;

pub fn run(settings: &AppSettings, bus: &Arc<Mutex<AppBus>>, db: Arc<DbPool>) -> Result<(), eframe::Error> {
    let icon_image_bytes = include_bytes!("../../images/icon.png");
    let icon_data = Arc::new(eframe::icon_data::from_png_bytes(icon_image_bytes).unwrap());

    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([1280.0, 768.0])
        .with_min_inner_size([800.0, 600.0])
        .with_icon(icon_data);

    let options = eframe::NativeOptions {
        vsync: true,
        viewport,

        ..Default::default()
    };

    let app_data = AppWin::new(settings, bus, db);

    eframe::run_native(
        "TF2 Monitor",
        options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);

            Ok(Box::new(app_data))

            // - eframe::run_native("My App", options, Box::new(|cc| Box::new(MyApp::new(cc))));
            // + eframe::run_native("My App", options, Box::new(|cc| Ok(Box::new(MyApp::new(cc)))));
        }),
    )
}

impl eframe::App for AppWin {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui_extras::install_image_loaders(ctx);
        colors::set_style(ctx);

        self.get_latest_lobby();

        self.friendship_positions.clear();

        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            add_top_menu(ui, self);
        });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            let image_desc = get_background_image_desc();

            add_status_row(self, ui, &image_desc);
        });

        egui::SidePanel::right("right_panel").show(ctx, |ui| {
            add_player_details_panel(self, ui);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // draw_background_image(ui);

            scoreboard::add_scoreboard(self, ui);

            ui.separator();

            ui.columns(2, |ui| {
                add_chat(&mut ui[0], self);
                add_kill_feed(&mut ui[1], self);
            });

            if self.app_settings.show_friendship_indicators {
                add_friendship_indicators(self, ui);
            }
        });

        // Show settings window if open
        show_settings_window(self, ctx);

        // Show DB statistics window if open
        show_db_statistics_window(self, ctx);

        sleep(GUI_SLEEP_DELAY);
        ctx.request_repaint_after(GUI_REPAINT_DELAY);
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        // eframe automatically saves persisted data from ctx.data_mut()
    }
}
