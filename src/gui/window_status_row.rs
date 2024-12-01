use super::background_image::ImageDescription;
use crate::models::AppWin;
use eframe::egui::{Align, Layout, Ui};

pub fn add_status_row(app_win: &AppWin, ui: &mut Ui, _image_desc: &ImageDescription) {
    ui.horizontal(|ui| {
        let lobby = &app_win.lobby;
        ui.label(format!(
            "Lobby: {} players, {} chat",
            lobby.players.len(),
            lobby.feed.len()
        ));

        // ui.label("Status: Online");

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label("Zoom with ctrl +/-");
            // add_image_creds(ui, image_desc);
        });
    });
}
