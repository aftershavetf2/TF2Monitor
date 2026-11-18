pub mod account_age;
pub mod background_image;
pub mod chat;
pub mod colors;
pub mod comments;
pub mod db_statistics_window;
pub mod friendship_indicators;
pub mod game_tab;
pub mod kill_feed;
pub mod markings;
pub mod menu_item;
pub mod overview_tab;
pub mod player_class_symbol;
pub mod player_database_window;
pub mod player_details_panel;
pub mod player_flag_editor;
mod player_notes;
pub mod player_tooltip;
pub mod playtime;
pub mod recently_left;
pub mod scoreboard;
pub mod scoreboard_team;
pub mod settings_window;
pub mod shortcut;
pub mod tabs;
pub mod theme;
pub mod tf2bd_lists_window;
pub mod title_bar;
pub mod top_menu;
pub mod ui_utils;
pub mod window_status_row;

use crate::{
    appbus::AppBus,
    config::GUI_REPAINT_DELAY,
    db::connection::DbPool,
    models::{AppWin, app_settings::AppSettings},
};
use background_image::get_background_image_desc;
use db_statistics_window::show_db_statistics_window;
use eframe::egui::{self};
use player_database_window::show_player_database_window;
use player_details_panel::{add_player_details_panel, show_player_details_windows};
use settings_window::show_settings_window;
use std::sync::{Arc, Mutex};
use window_status_row::add_status_row;

pub fn run(
    settings: &AppSettings,
    bus: &Arc<Mutex<AppBus>>,
    db: Arc<DbPool>,
) -> Result<(), eframe::Error> {
    let icon_image_bytes = include_bytes!("../../images/icon.png");
    let icon_data = Arc::new(eframe::icon_data::from_png_bytes(icon_image_bytes).unwrap());

    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([1280.0, 768.0])
        .with_min_inner_size([800.0, 600.0])
        .with_decorations(false)
        .with_icon(icon_data);

    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport,

        ..Default::default()
    };

    let mut app_data = AppWin::new(settings, bus, db);
    app_data.tf2bd_lists = crate::tf2bd::player_lists::PlayerLists::load();

    eframe::run_native(
        "TF2 Monitor",
        options,
        Box::new(|cc| {
            theme::install(&cc.egui_ctx);
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);

            Ok(Box::new(app_data))
        }),
    )
}

impl eframe::App for AppWin {
    fn ui(&mut self, root_ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = &root_ui.ctx().clone();
        self.get_latest_lobby();
        self.tf2bd_lists.poll();
        self.tf2bd_lists.check_for_updates();

        self.friendship_positions.clear();

        top_menu::handle_shortcuts(ctx, self);
        title_bar::show(root_ui, self);

        let status = egui::Panel::bottom("status")
            .show_separator_line(false)
            .show(root_ui, |ui| {
                let image_desc = get_background_image_desc();

                add_status_row(self, ui, &image_desc);
            });

        theme::panel_separator(ctx, &status.response, theme::PanelEdge::Top);

        let details = egui::Panel::right("right_panel")
            .show_separator_line(false)
            .show(root_ui, |ui| {
                add_player_details_panel(self, ui);
            });

        theme::panel_separator(ctx, &details.response, theme::PanelEdge::Left);

        egui::CentralPanel::default().show(root_ui, |ui| {
            tabs::add_tab_bar(&mut self.active_tab, ui);
            ui.separator();
            match self.active_tab {
                tabs::Tab::Game => game_tab::show(self, ui),
                tabs::Tab::Overview => overview_tab::show(self, ui),
            }
        });

        // Show settings window if open
        show_settings_window(self, ctx);

        // Show DB statistics window if open
        show_db_statistics_window(self, ctx);

        // Show player database window if open
        show_player_database_window(self, ctx);
        tf2bd_lists_window::show(self, ctx);

        // Show separate player detail windows
        show_player_details_windows(self, ctx);

        title_bar::resize_edges(ctx);

        ctx.request_repaint_after(GUI_REPAINT_DELAY);
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        // eframe automatically saves persisted data from ctx.data_mut()
    }
}

#[cfg(test)]
mod tests;
