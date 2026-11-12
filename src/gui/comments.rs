use crate::tf2::{lobby::Player, steamapi::SteamProfileComment};
use eframe::egui::{self, Color32, OpenUrl, RichText, ScrollArea, Sense, Ui};

pub fn add_profile_comments(player: &Player, ui: &mut Ui) {
    ui.vertical(|ui| {
        ui.heading("Profile Comments");

        if let Some(comments) = &player.profile_comments {
            ui.push_id("comments", |ui| {
                // Wrapped comment bodies have variable heights, so lay out complete entries.
                ScrollArea::vertical()
                    .stick_to_bottom(false)
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        for (index, comment) in comments.iter().enumerate() {
                            ui.push_id(index, |ui| add_comment_row(ui, comment));
                        }
                    });
            });
        } else {
            ui.label("Loading comments...");
        }
    });
}

fn add_comment_row(ui: &mut Ui, row: &SteamProfileComment) {
    ui.vertical(|ui| {
        ui.horizontal_top(|ui| {
            let avatar = if row.avatar_thumb.is_empty() {
                let (rect, response) =
                    ui.allocate_exact_size(egui::vec2(24.0, 24.0), Sense::click());
                ui.painter()
                    .rect_filled(rect, 3.0, super::theme::IMAGE_BACKGROUND);
                response
            } else {
                super::theme::image(
                    ui,
                    egui::Image::new(&row.avatar_thumb)
                        .fit_to_exact_size(egui::vec2(24.0, 24.0))
                        .corner_radius(3.0)
                        .sense(Sense::click()),
                )
            };
            let name = ui
                .vertical(|ui| {
                    ui.set_width(ui.available_width());
                    ui.spacing_mut().item_spacing.y = 2.0;
                    let name = ui.add(
                        egui::Label::new(RichText::new(&row.name).color(Color32::WHITE))
                            .wrap()
                            .sense(Sense::click()),
                    );
                    ui.add(
                        egui::Label::new(RichText::new(&row.comment).color(super::theme::TEXT))
                            .wrap(),
                    );
                    name
                })
                .inner;
            let response = avatar
                .union(name)
                .on_hover_text("View player on Steam History");
            if response.clicked() {
                ui.ctx().open_url(OpenUrl {
                    url: row.steamid.steam_history_url(),
                    new_tab: true,
                });
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::steamid::SteamID;

    #[test]
    fn comments_wrap_below_author_and_before_next_entry() {
        let ctx = egui::Context::default();
        super::super::theme::install(&ctx);
        let output = ctx.run(egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(300.0,600.0))),
            ..Default::default()
        }, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                for name in ["First Author", "Second Author"] {
                    add_comment_row(ui, &SteamProfileComment {
                        name: name.into(), avatar_thumb: String::new(),
                        steamid: SteamID::from_u64(1),
                        comment: "A long comment that wraps onto multiple lines in the narrow profile panel.".into(),
                    });
                }
            });
        });
        let text: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text),
                _ => None,
            })
            .collect();
        assert_eq!(text.len(), 4);
        assert_eq!(text[0].galley.text(), "First Author");
        assert_eq!(text[2].galley.text(), "Second Author");
        for (author, body) in [(text[0], text[1]), (text[2], text[3])] {
            assert!(body.pos.y > author.pos.y + author.galley.size().y);
            assert!(body.galley.rows.len() > 1);
            assert!((author.pos.x - body.pos.x).abs() < 0.01);
            assert!(
                body.pos.x >= 24.0 + 8.0,
                "Text must stay beside the avatar column"
            );
            assert_eq!(
                body.galley.job.sections[0].format.color,
                super::super::theme::TEXT
            );
            assert_ne!(body.galley.job.sections[0].format.color, Color32::WHITE);
        }
        assert!(text[2].pos.y > text[1].pos.y + text[1].galley.size().y);
    }
}
