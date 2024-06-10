use super::{background_image::ImageDescription, image_creds::add_image_creds};
use crate::models::AppWin;
use eframe::egui::{Align, Layout, Ui};

pub fn add_status_row(app_win: &AppWin, ui: &mut Ui, image_desc: &ImageDescription) {
    ui.horizontal(|ui| {
        let lobby = &app_win.lobby;
        ui.label(format!(
            "Lobby: {} players, {} chat",
            lobby.players.len(),
            lobby.chat.len()
        ));

        ui.label("Zoom with ctrl +/-");

        // ui.label("Status: Online");

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            add_image_creds(ui, image_desc);
        });
    });
}
