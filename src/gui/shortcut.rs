//! Painted keycaps share their menu row's click target.
use eframe::egui::{self, Color32, Rect, Stroke, StrokeKind};

pub fn command_label() -> &'static str {
    if cfg!(target_os = "macos") {
        "Cmd"
    } else {
        "Ctrl"
    }
}

pub fn width(ui: &egui::Ui, keys: &[&str]) -> f32 {
    keys.iter()
        .map(|key| galley(ui, key).size().x.max(8.0) + 12.0)
        .sum::<f32>()
        + keys.len().saturating_sub(1) as f32 * 15.0
}

fn galley(ui: &egui::Ui, text: &str) -> std::sync::Arc<egui::Galley> {
    ui.painter()
        .layout_no_wrap(text.into(), super::theme::shortcut_font(), Color32::BLACK)
}

pub fn paint(ui: &egui::Ui, rect: Rect, keys: &[&str]) {
    let mut x = rect.right() - width(ui, keys);
    for (index, key) in keys.iter().enumerate() {
        if index > 0 {
            ui.painter().text(
                egui::pos2(x + 7.5, rect.center().y),
                egui::Align2::CENTER_CENTER,
                "+",
                super::theme::shortcut_font(),
                Color32::WHITE,
            );
            x += 15.0;
        }
        let text = galley(ui, key);
        let base = Rect::from_min_size(
            egui::pos2(x, rect.center().y - 10.0),
            egui::vec2((text.size().x + 12.0).max(20.0), 20.0),
        );
        ui.painter().rect_filled(base, 3.0, Color32::from_gray(166));
        let face = Rect::from_min_max(base.min, base.max - egui::vec2(0.0, 2.0));
        ui.painter().rect(
            face,
            3.0,
            Color32::from_gray(244),
            Stroke::new(1.0_f32, Color32::WHITE),
            StrokeKind::Inside,
        );
        ui.painter()
            .galley(face.center() - text.size() / 2.0, text, Color32::BLACK);
        x = base.right();
    }
}

/// Consume app commands only when no editor or modal owns keyboard input.
pub fn consume_command(ctx: &egui::Context, key: egui::Key) -> bool {
    if ctx.wants_keyboard_input() || ctx.memory(|m| m.top_modal_layer().is_some()) {
        return false;
    }
    ctx.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, key))
}
