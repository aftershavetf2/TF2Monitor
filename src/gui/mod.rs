pub mod chat;
pub mod colors;
pub mod player_tooltip;
pub mod scoreboard;
pub mod scoreboard_team;

use crate::{
    appbus::AppBus,
    models::{app_settings::AppSettings, AppWin},
};
use eframe::egui;
use std::sync::{Arc, Mutex};

pub fn run(settings: &AppSettings, bus: &Arc<Mutex<AppBus>>) -> Result<(), eframe::Error> {
    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([800.0, 600.0])
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

        // add_menu_row(ctx);

        // add_status_row(ctx);

        // scoreboard::make_scoreboard(ctx, &self.scores);

        egui::CentralPanel::default().show(ctx, |ui| {
            scoreboard::add_scoreboard(
                ui,
                self.self_steamid,
                &mut self.lobby,
                &mut self.swap_team_colors,
                &mut self.show_crits,
            );

            ui.separator();

            chat::add_chat(ui, &self.lobby, &mut self.swap_team_colors);
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
