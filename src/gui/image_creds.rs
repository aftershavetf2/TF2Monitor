use eframe::egui::Color32;

use super::background_image::ImageDescription;

pub fn add_image_creds(ui: &mut eframe::egui::Ui, image_desc: &ImageDescription) {
    ui.horizontal(|ui| {
        ui.colored_label(Color32::GRAY, "Image by");
        ui.hyperlink_to(&image_desc.author, &image_desc.url);
    });
}
