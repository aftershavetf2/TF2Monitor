use eframe::egui::{Color32, Ui};

/// Display an empty/missing value indicator
pub fn show_empty_value(ui: &mut Ui) {
    ui.colored_label(Color32::GRAY, " ");
}
