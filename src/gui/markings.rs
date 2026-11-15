use super::colors::color_for_flag;
use crate::{
    tf2::lobby::{Player, player_attribute_description},
    tf2bd::models::PlayerAttribute,
};
use eframe::egui::{self, Color32, FontId, Response, Sense, TextStyle, Ui};

const BADGE_FONT_SIZE: f32 = 10.0;
const BADGE_PADDING: egui::Vec2 = egui::vec2(6.0, 4.0);
const BADGE_CORNER_RADIUS: f32 = 3.0;

pub(super) fn add_sourceban_row(
    ui: &mut Ui,
    ban: &crate::reputation::SourceBan,
    today: chrono::NaiveDate,
) {
    let font = TextStyle::Body.resolve(ui.style());
    let mut text = egui::text::LayoutJob::default();
    text.append(
        &ban.source,
        0.0,
        egui::TextFormat {
            font_id: font.clone(),
            color: Color32::WHITE,
            ..Default::default()
        },
    );
    text.append(
        &format!(
            " [{}]:\n{}",
            ban.record.status_on(today),
            ban.record.reason.as_deref().unwrap_or("Unknown reason")
        ),
        0.0,
        egui::TextFormat {
            font_id: font,
            color: super::theme::TEXT,
            ..Default::default()
        },
    );
    ui.add(egui::Label::new(text).wrap());
}

pub fn add_reputation(ui: &mut Ui, player: &Player) {
    add_team_symbol_basic_rep(ui, player);
    add_flags(ui, player);

    // Indicate available ban history; users assess its relevance.
    if let Some(reputation) = &player.reputation {
        if !reputation.source_bans.is_empty() {
            let warning = reputation.has_warning_ban();
            let response = if warning {
                add_sourceban_warning(ui)
            } else {
                colored_box(
                    ui,
                    "SB",
                    Color32::BLACK,
                    Color32::ORANGE,
                    FontId::proportional(BADGE_FONT_SIZE),
                )
            };
            response.on_hover_ui(|ui| {
                ui.label("SourceBans");
                if warning {
                    ui.label("Warning keyword found in ban history.");
                    ui.add_space(4.0);
                }
                let today = chrono::Utc::now().date_naive();
                for ban in &reputation.source_bans {
                    add_sourceban_row(ui, ban, today);
                }
            });
        }
    }
}

/// Keep the text and warning together inside a single badge.
/// Paint the warning directly so it does not depend on font symbol coverage.
fn add_sourceban_warning(ui: &mut Ui) -> Response {
    let text = ui.painter().layout_no_wrap(
        "SB".into(),
        FontId::proportional(BADGE_FONT_SIZE),
        Color32::BLACK,
    );
    let icon_size = 12.0;
    let gap = 4.0;
    let size = egui::vec2(
        text.size().x + gap + icon_size,
        text.size().y.max(icon_size),
    ) + 2.0 * BADGE_PADDING;
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Label, ui.is_enabled(), "SB warning")
    });
    if ui.is_rect_visible(rect) {
        ui.painter()
            .rect_filled(rect, BADGE_CORNER_RADIUS, Color32::ORANGE);
        let text_pos = egui::pos2(
            rect.left() + BADGE_PADDING.x,
            rect.center().y - text.size().y / 2.0,
        );
        let triangle = egui::Rect::from_center_size(
            egui::pos2(
                text_pos.x + text.size().x + gap + icon_size / 2.0,
                rect.center().y,
            ),
            egui::Vec2::splat(icon_size),
        );
        ui.painter().galley(text_pos, text, Color32::BLACK);
        let center_x = triangle.center().x;
        ui.painter().add(egui::Shape::convex_polygon(
            vec![
                triangle.center_top(),
                triangle.right_bottom(),
                triangle.left_bottom(),
            ],
            Color32::ORANGE,
            egui::Stroke::new(1.0, Color32::BLACK),
        ));
        ui.painter().line_segment(
            [
                egui::pos2(center_x, triangle.top() + icon_size * 0.35),
                egui::pos2(center_x, triangle.top() + icon_size * 0.65),
            ],
            egui::Stroke::new(1.4, Color32::BLACK),
        );
        ui.painter().circle_filled(
            egui::pos2(center_x, triangle.top() + icon_size * 0.84),
            0.8,
            Color32::BLACK,
        );
    }
    response
}

pub fn add_flags(ui: &mut Ui, player: &Player) {
    let player_attributes_to_show = vec![
        PlayerAttribute::Cool,
        PlayerAttribute::Cheater,
        PlayerAttribute::Bot,
        PlayerAttribute::Suspicious,
        PlayerAttribute::Toxic,
        PlayerAttribute::Exploiter,
    ];

    if let Some(player_info) = &player.player_info {
        for player_attribute in player_attributes_to_show {
            if player_info.attributes.contains(&player_attribute) {
                add_flag(ui, player_attribute);
            }
        }
    }
}

fn add_flag(ui: &mut Ui, player_attribute: PlayerAttribute) {
    let (fgcolor, bgcolor) = color_for_flag(player_attribute);

    let (text, tooltip) = player_attribute_description(player_attribute);

    add_badge(ui, text, fgcolor, bgcolor, tooltip);
}

fn add_team_symbol_basic_rep(ui: &mut Ui, player: &Player) {
    if let Some(tooltip) = &player.has_vac_bans() {
        add_badge(ui, "VAC", Color32::WHITE, Color32::DARK_RED, tooltip);
    }

    if let Some(tooltip) = &player.has_game_bans() {
        add_badge(ui, "GB", Color32::WHITE, Color32::DARK_RED, tooltip);
    }
}

fn add_badge(ui: &mut Ui, text: &str, fgcolor: Color32, bgcolor: Color32, tooltip: &str) {
    colored_box(
        ui,
        text,
        fgcolor,
        bgcolor,
        FontId::proportional(BADGE_FONT_SIZE),
    )
    .on_hover_text(tooltip);
}

pub fn add_new_player_value(ui: &mut Ui, text: String, highlight: bool) -> Response {
    if highlight {
        colored_box(
            ui,
            &text,
            Color32::WHITE,
            Color32::DARK_GREEN,
            TextStyle::Body.resolve(ui.style()),
        )
    } else {
        ui.label(text)
    }
}

fn colored_box(
    ui: &mut Ui,
    text: &str,
    fgcolor: Color32,
    bgcolor: Color32,
    font: FontId,
) -> Response {
    let text = ui.painter().layout_no_wrap(text.to_owned(), font, fgcolor);
    // Allocate directly in the wrapping row so each badge stays together.
    let (rect, response) =
        ui.allocate_exact_size(text.size() + 2.0 * BADGE_PADDING, Sense::hover());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Label, ui.is_enabled(), text.text())
    });
    if ui.is_rect_visible(rect) {
        ui.painter().rect_filled(rect, BADGE_CORNER_RADIUS, bgcolor);
        let text_pos = rect.center() - text.size() / 2.0;
        ui.painter().galley(text_pos, text, fgcolor);
    }
    response
}
