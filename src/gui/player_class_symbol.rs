use crate::tf2::{
    lobby::Player,
    weapon_classes::{PlayerClass, weapon_class},
};
use eframe::egui::{self, Align2, Color32, FontId, Image, Response, Sense, Ui};

const KILL_LOOKBACK: usize = 5;
const SYMBOL_SIZE: f32 = 25.0;

fn inferred_class(player: &Player) -> Option<PlayerClass> {
    player
        .kills_with
        .iter()
        .rev()
        .take(KILL_LOOKBACK)
        .find_map(|kill| weapon_class(&kill.weapon))
}

/// Draw the estimated class based on the most recent unambiguous weapon.
pub fn add_player_class_symbol(ui: &mut Ui, player: &Player) -> Response {
    let class = inferred_class(player);
    let (rect, response) = ui.allocate_exact_size(egui::Vec2::splat(SYMBOL_SIZE), Sense::hover());
    let tooltip = match class {
        Some(class) => format!("Estimated class: {class:?}\nBased on the last five kills"),
        None => "Unknown class: no unambiguous weapon in the last five kills".to_owned(),
    };
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, ui.is_enabled(), &tooltip));
    if ui.is_rect_visible(rect) {
        if let Some(class) = class {
            Image::new(class_icon_url(class)).paint_at(ui, rect);
        } else {
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                "?",
                FontId::proportional(20.0),
                Color32::WHITE,
            );
        }
    }
    response.on_hover_text(tooltip)
}

fn class_icon_url(class: PlayerClass) -> &'static str {
    match class {
        PlayerClass::Scout => {
            "https://wiki.teamfortress.com/w/images/thumb/a/ad/Leaderboard_class_scout.png/48px-Leaderboard_class_scout.png"
        }
        PlayerClass::Soldier => {
            "https://wiki.teamfortress.com/w/images/thumb/9/96/Leaderboard_class_soldier.png/48px-Leaderboard_class_soldier.png"
        }
        PlayerClass::Pyro => {
            "https://wiki.teamfortress.com/w/images/thumb/8/80/Leaderboard_class_pyro.png/48px-Leaderboard_class_pyro.png"
        }
        PlayerClass::Demoman => {
            "https://wiki.teamfortress.com/w/images/thumb/4/47/Leaderboard_class_demoman.png/48px-Leaderboard_class_demoman.png"
        }
        PlayerClass::Heavy => {
            "https://wiki.teamfortress.com/w/images/thumb/5/5a/Leaderboard_class_heavy.png/48px-Leaderboard_class_heavy.png"
        }
        PlayerClass::Engineer => {
            "https://wiki.teamfortress.com/w/images/thumb/1/12/Leaderboard_class_engineer.png/48px-Leaderboard_class_engineer.png"
        }
        PlayerClass::Medic => {
            "https://wiki.teamfortress.com/w/images/thumb/e/e5/Leaderboard_class_medic.png/48px-Leaderboard_class_medic.png"
        }
        PlayerClass::Sniper => {
            "https://wiki.teamfortress.com/w/images/thumb/f/fe/Leaderboard_class_sniper.png/48px-Leaderboard_class_sniper.png"
        }
        PlayerClass::Spy => {
            "https://wiki.teamfortress.com/w/images/thumb/3/33/Leaderboard_class_spy.png/48px-Leaderboard_class_spy.png"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tf2::lobby::PlayerKill;
    use egui::load::{ImageLoadResult, ImageLoader, ImagePoll, LoadError, SizeHint};
    use std::sync::Arc;

    struct SoldierIconLoader;

    impl ImageLoader for SoldierIconLoader {
        fn id(&self) -> &str {
            "test_soldier_icon"
        }

        fn load(&self, _: &egui::Context, uri: &str, _: SizeHint) -> ImageLoadResult {
            if uri
                != "https://wiki.teamfortress.com/w/images/thumb/9/96/Leaderboard_class_soldier.png/48px-Leaderboard_class_soldier.png"
            {
                return Err(LoadError::NotSupported);
            }
            Ok(ImagePoll::Ready {
                image: Arc::new(egui::ColorImage::filled([48, 48], Color32::WHITE)),
            })
        }

        fn forget(&self, _: &str) {}
        fn forget_all(&self) {}
        fn byte_size(&self) -> usize {
            0
        }
    }

    fn player(weapons: &[&str]) -> Player {
        Player {
            kills_with: weapons
                .iter()
                .map(|weapon| PlayerKill {
                    weapon: (*weapon).to_owned(),
                    crit: false,
                })
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn scoreboard_draws_the_class_image_and_marks_unknown_classes() {
        let ctx = egui::Context::default();
        super::super::theme::install(&ctx);
        ctx.add_image_loader(Arc::new(SoldierIconLoader));
        let output = super::super::tests::run_ui(&ctx, Default::default(), |ui| {
            let known = add_player_class_symbol(ui, &player(&["rocketlauncher"]));
            let unknown = add_player_class_symbol(ui, &player(&[]));
            assert_eq!(known.rect.size(), egui::Vec2::splat(SYMBOL_SIZE));
            assert_eq!(unknown.rect.size(), known.rect.size());
        });
        assert!(output.shapes.iter().any(|shape| {
            matches!(&shape.shape, egui::Shape::Rect(rect) if rect.brush.is_some())
        }));
        assert!(output.shapes.iter().any(|shape| {
            matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text() == "?")
        }));
    }

    #[test]
    fn newest_unambiguous_kill_wins() {
        assert_eq!(
            inferred_class(&player(&["rocketlauncher", "iron_bomber"])),
            Some(PlayerClass::Demoman)
        );
        assert_eq!(
            inferred_class(&player(&["wrench", "pistol", "shotgun", "custom_weapon"])),
            Some(PlayerClass::Engineer)
        );
    }

    #[test]
    fn searches_exactly_five_recent_kills() {
        assert_eq!(
            inferred_class(&player(&["knife", "pistol", "shotgun", "world", "pistol"])),
            Some(PlayerClass::Spy)
        );
        assert_eq!(
            inferred_class(&player(&[
                "knife", "pistol", "shotgun", "world", "pistol", "shotgun"
            ])),
            None
        );
        assert_eq!(inferred_class(&player(&[])), None);
        assert_eq!(inferred_class(&player(&["pistol", "shotgun"])), None);
    }
}
