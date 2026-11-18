//! Full-row menu targets with a shared icon gutter and shortcut column.
use eframe::egui::{self, Color32, Response, Sense, StrokeKind, Ui};

fn row(ui: &mut Ui, label: &str, selected: Option<bool>, keys: &[&str]) -> Response {
    let text = ui.painter().layout_no_wrap(
        label.into(),
        egui::TextStyle::Button.resolve(ui.style()),
        Color32::WHITE,
    );
    let shortcut_width = if keys.is_empty() {
        0.0
    } else {
        super::shortcut::width(ui, keys) + 24.0
    };
    let width = ui
        .available_width()
        .max(220.0)
        .max(20.0 + 14.0 + 8.0 + text.size().x + shortcut_width);
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(width, if keys.is_empty() { 27.0 } else { 30.0 }),
        Sense::click(),
    );
    let visuals = ui.style().interact(&response);
    ui.painter().rect_filled(rect, 4.0, visuals.weak_bg_fill);
    let color = if ui.is_enabled() {
        visuals.fg_stroke.color
    } else {
        ui.visuals().weak_text_color()
    };
    let icon = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 17.0, rect.center().y),
        egui::Vec2::splat(super::theme::CHECKBOX_SIZE),
    );
    if let Some(checked) = selected {
        let frame = super::theme::checkbox_frame(color);
        ui.painter().rect(
            icon,
            frame.corner_radius,
            frame.fill,
            frame.stroke,
            StrokeKind::Inside,
        );
        if checked {
            let check =
                icon.shrink((super::theme::CHECKBOX_SIZE - super::theme::CHECKMARK_SIZE) / 2.0);
            ui.painter().add(egui::Shape::line(
                vec![
                    check.left_center(),
                    check.center_bottom(),
                    check.right_top(),
                ],
                super::theme::CHECKMARK_STROKE,
            ));
        }
    }
    ui.painter().galley_with_override_text_color(
        egui::pos2(icon.right() + 8.0, rect.center().y - text.size().y / 2.0),
        text,
        color,
    );
    super::shortcut::paint(ui, rect.shrink2(egui::vec2(10.0, 0.0)), keys);
    response.widget_info(|| match selected {
        Some(checked) => {
            egui::WidgetInfo::selected(egui::Role::CheckBox, ui.is_enabled(), checked, label)
        }
        None => egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), label),
    });
    response
}

pub fn command(ui: &mut Ui, label: &str) -> Response {
    command_with_keys(ui, label, &[])
}

pub fn command_with_keys(ui: &mut Ui, label: &str, keys: &[&str]) -> Response {
    let response = row(ui, label, None, keys);
    if response.clicked() {
        ui.close();
    }
    response
}

pub fn checkbox(ui: &mut Ui, checked: &mut bool, label: &str) -> Response {
    let mut response = row(ui, label, Some(*checked), &[]);
    if response.clicked() {
        *checked = !*checked;
        response.mark_changed();
    }
    response
}

/// Keep submenu labels in the same text column as commands and checkboxes.
pub fn submenu<R>(ui: &mut Ui, label: &str, contents: impl FnOnce(&mut Ui) -> R) {
    let response = row(ui, label, None, &[]);
    let arrow_rect = egui::Rect::from_center_size(
        egui::pos2(response.rect.right() - 17.0, response.rect.center().y),
        egui::Vec2::splat(14.0),
    );
    ui.painter()
        .add(egui::containers::menu::SubMenuButton::arrow_shape(
            arrow_rect,
            ui.style().interact(&response).fg_stroke.color,
        ));
    egui::containers::menu::SubMenu::new().show(ui, &response, contents);
}

/// Inset wrapped menu information to match the command text column.
pub fn information(ui: &mut Ui, contents: impl FnOnce(&mut Ui)) {
    egui::Frame::NONE
        .inner_margin(egui::Margin {
            left: 32,
            right: 10,
            top: 6,
            bottom: 6,
        })
        .show(ui, |ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
            contents(ui);
        });
}
