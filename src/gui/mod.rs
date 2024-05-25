pub mod background_image;
pub mod chat;
pub mod colors;
pub mod friendship_indicators;
pub mod image_creds;
pub mod player_menu;
pub mod player_tooltip;
pub mod recently_left;
pub mod scoreboard;
pub mod scoreboard_team;

use self::friendship_indicators::add_friendship_indicators;
use crate::{
    appbus::AppBus,
    models::{app_settings::AppSettings, AppWin},
};
use eframe::egui;
use std::sync::{Arc, Mutex};

pub fn run(settings: &AppSettings, bus: &Arc<Mutex<AppBus>>) -> Result<(), eframe::Error> {
    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([1024.0, 768.0])
        .with_min_inner_size([800.0, 600.0]);

    let options = eframe::NativeOptions {
        vsync: true,
        viewport,

        ..Default::default()
    };

    let app_data = AppWin::new(settings, bus);

    eframe::run_native(
        "TF2 Monitor",
        options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);

            Box::new(app_data)
        }),
    )
}

impl eframe::App for AppWin {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui_extras::install_image_loaders(ctx);
        colors::set_style(ctx);

        self.process_bus();

        self.friendship_positions.clear();

        egui::CentralPanel::default().show(ctx, |ui| {
            scoreboard::add_scoreboard(self, ui);

            ui.separator();

            chat::add_chat(ui, &self.lobby, &mut self.swap_team_colors);

            if self.show_friendships {
                add_friendship_indicators(self, ui);
            }
        });

        ctx.request_repaint();
    }
}

// fn add_menu_row(ctx: &egui::Context) {
//     egui::TopBottomPanel::top("menu").show(ctx, |ui| {
//         ui.label("Menus...");
//     });
// }

// fn add_status_row(ctx: &egui::Context) {
//     egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
//         ui.label("Status: Online");
//     });
// }
