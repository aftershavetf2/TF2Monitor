use super::{background_image::ImageDescription, image_creds::add_image_creds};
use eframe::egui::{Align, Layout, Ui};

pub fn add_status_row(ui: &mut Ui, image_desc: &ImageDescription) {
    ui.horizontal(|ui| {
        // ui.label("Status: Online");

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            add_image_creds(ui, image_desc);
        });
    });
}
