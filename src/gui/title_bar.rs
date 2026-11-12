//! Native window commands behind a custom, undecorated title bar.
use crate::models::AppWin;
use eframe::egui::{self, CursorIcon, ResizeDirection, Sense, ViewportCommand};

pub fn show(ctx: &egui::Context, app: &mut AppWin) {
    let panel = egui::TopBottomPanel::top("title_bar")
        .show_separator_line(false)
        .frame(
            egui::Frame::new()
                .fill(super::theme::PANEL)
                .inner_margin(egui::Margin {
                    left: 8,
                    right: 0,
                    top: 2,
                    bottom: 2,
                }),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                super::top_menu::add_top_menu(ui, app);
            });
        });
    super::theme::panel_separator(ctx, &panel.response, super::theme::PanelEdge::Bottom);
}

pub fn contents(ui: &mut egui::Ui, title: &str) -> [egui::Response; 3] {
    let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
    let height = 26.0;
    let width = (ui.available_width() - 126.0 - ui.spacing().item_spacing.x).max(0.0);
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width, height), Sense::click_and_drag());
    let center = egui::pos2(ui.ctx().content_rect().center().x, rect.center().y);
    let text_rect = rect.shrink2(egui::vec2(12.0, 0.0));
    let half_width = (center.x - text_rect.left())
        .min(text_rect.right() - center.x)
        .max(0.0);
    if half_width > 0.0 {
        let mut job = egui::text::LayoutJob::simple(
            title.to_owned(),
            egui::TextStyle::Body.resolve(ui.style()),
            ui.visuals().text_color(),
            half_width * 2.0,
        );
        job.wrap.max_rows = 1;
        job.wrap.break_anywhere = true;
        let text = ui.painter().layout_job(job);
        ui.painter()
            .galley(center - text.size() / 2.0, text, ui.visuals().text_color());
    }
    if response.double_clicked_by(egui::PointerButton::Primary) {
        ui.ctx()
            .send_viewport_cmd(ViewportCommand::Maximized(!maximized));
    } else if response.drag_started_by(egui::PointerButton::Primary) {
        // Wait for movement so the native drag loop does not consume clicks
        // before egui can recognize a double-click.
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }
    // Updating the native title also keeps task switchers and accessibility in sync.
    let changed = ui.ctx().data_mut(|data| {
        let id = egui::Id::new("native_window_title");
        if data.get_temp::<String>(id).as_deref() != Some(title) {
            data.insert_temp(id, title.to_owned());
            true
        } else {
            false
        }
    });
    if changed {
        ui.ctx()
            .send_viewport_cmd(ViewportCommand::Title(title.into()));
    }
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        [
            ("−", "Minimize", ViewportCommand::Minimized(true)),
            (
                "",
                if maximized { "Restore" } else { "Maximize" },
                ViewportCommand::Maximized(!maximized),
            ),
            ("×", "Close", ViewportCommand::Close),
        ]
        .map(|(symbol, name, command)| {
            let (rect, response) = ui.allocate_exact_size(egui::vec2(42.0, height), Sense::click());
            if response.hovered() || response.has_focus() || response.is_pointer_button_down_on() {
                ui.painter().rect_filled(rect, 0.0, super::theme::HOVER);
            }
            if symbol.is_empty() {
                paint_maximize_icon(ui.painter(), rect, egui::Color32::WHITE, maximized);
            } else {
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    symbol,
                    egui::FontId::proportional(if symbol == "×" { 17.0 } else { 13.0 }),
                    egui::Color32::WHITE,
                );
            }
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), name)
            });
            if response.clicked() {
                ui.ctx().send_viewport_cmd(command);
            }
            response.on_hover_text(name)
        })
    })
    .inner
}

// Draw window outlines directly so the restore icon does not depend on font glyph coverage.
fn paint_maximize_icon(
    painter: &egui::Painter,
    rect: egui::Rect,
    color: egui::Color32,
    maximized: bool,
) {
    let stroke = egui::Stroke::new(1.0_f32, color);
    let center = rect.center();
    if maximized {
        let back =
            egui::Rect::from_center_size(center + egui::vec2(2.0, -2.0), egui::vec2(10.0, 10.0));
        painter.add(egui::Shape::line(
            vec![
                back.left_top() + egui::vec2(0.0, 3.0),
                back.left_top(),
                back.right_top(),
                back.right_bottom(),
                back.right_bottom() - egui::vec2(3.0, 0.0),
            ],
            stroke,
        ));
    }
    let offset = if maximized {
        egui::vec2(-2.0, 2.0)
    } else {
        egui::Vec2::ZERO
    };
    painter.rect_stroke(
        egui::Rect::from_center_size(center + offset, egui::vec2(10.0, 10.0)),
        0.0,
        stroke,
        egui::StrokeKind::Inside,
    );
}

/// Undecorated windows need resize targets along their edges and corners.
pub fn resize_edges(ctx: &egui::Context) {
    if ctx.memory(|m| m.top_modal_layer().is_some()) {
        return;
    }
    if ctx.input(|input| {
        input.viewport().maximized.unwrap_or(false) || input.viewport().fullscreen.unwrap_or(false)
    }) {
        return;
    }
    let rect = ctx.content_rect();
    let edge = 5.0;
    let corner = 10.0;
    let x = [
        rect.left(),
        rect.left() + corner,
        rect.right() - corner,
        rect.right(),
    ];
    let y = [
        rect.top(),
        rect.top() + corner,
        rect.bottom() - corner,
        rect.bottom(),
    ];
    for (index, (target, direction, cursor)) in [
        (
            egui::Rect::from_min_max(egui::pos2(x[0], y[0]), egui::pos2(x[1], y[1])),
            ResizeDirection::NorthWest,
            CursorIcon::ResizeNwSe,
        ),
        (
            egui::Rect::from_min_max(egui::pos2(x[2], y[0]), egui::pos2(x[3], y[1])),
            ResizeDirection::NorthEast,
            CursorIcon::ResizeNeSw,
        ),
        (
            egui::Rect::from_min_max(egui::pos2(x[0], y[2]), egui::pos2(x[1], y[3])),
            ResizeDirection::SouthWest,
            CursorIcon::ResizeNeSw,
        ),
        (
            egui::Rect::from_min_max(egui::pos2(x[2], y[2]), egui::pos2(x[3], y[3])),
            ResizeDirection::SouthEast,
            CursorIcon::ResizeNwSe,
        ),
        (
            egui::Rect::from_min_max(egui::pos2(x[1], y[0]), egui::pos2(x[2], y[0] + edge)),
            ResizeDirection::North,
            CursorIcon::ResizeVertical,
        ),
        (
            egui::Rect::from_min_max(egui::pos2(x[1], y[3] - edge), egui::pos2(x[2], y[3])),
            ResizeDirection::South,
            CursorIcon::ResizeVertical,
        ),
        (
            egui::Rect::from_min_max(egui::pos2(x[0], y[1]), egui::pos2(x[0] + edge, y[2])),
            ResizeDirection::West,
            CursorIcon::ResizeHorizontal,
        ),
        (
            egui::Rect::from_min_max(egui::pos2(x[3] - edge, y[1]), egui::pos2(x[3], y[2])),
            ResizeDirection::East,
            CursorIcon::ResizeHorizontal,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        egui::Area::new(egui::Id::new(("window_resize", index)))
            .order(egui::Order::Foreground)
            .fixed_pos(target.min)
            .movable(false)
            .constrain(false)
            .show(ctx, |ui| {
                let (_, response) = ui.allocate_exact_size(target.size(), Sense::drag());
                if response.hovered() || response.is_pointer_button_down_on() {
                    ctx.set_cursor_icon(cursor);
                    if ui.input(|input| input.pointer.primary_pressed()) {
                        ctx.send_viewport_cmd(ViewportCommand::BeginResize(direction));
                    }
                }
            });
    }
}
