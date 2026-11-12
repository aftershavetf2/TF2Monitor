use eframe::egui::Ui;

/// Display an empty/missing value indicator
pub fn show_empty_value(ui: &mut Ui) {
    ui.colored_label(super::theme::muted_text(), " ");
}
