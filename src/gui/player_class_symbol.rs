use crate::tf2::{
    lobby::Player,
    weapon_classes::{PlayerClass, weapon_class},
};
use eframe::egui::{self, Align2, Color32, FontFamily, FontId, Response, Sense, Ui};

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

/// Draw an estimated class in white, or `?` when the last five kills
/// contain no unambiguous weapon. This is not a live class observation.
pub fn add_player_class_symbol(ui: &mut Ui, player: &Player) -> Response {
    let class = inferred_class(player);
    let (rect, response) = ui.allocate_exact_size(egui::Vec2::splat(SYMBOL_SIZE), Sense::hover());
    let tooltip = match class {
        Some(class) => format!("Estimated class: {class:?}\nBased on the last five kills"),
        None => "Unknown class: no unambiguous weapon in the last five kills".to_owned(),
    };
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Label, ui.is_enabled(), &tooltip)
    });
    if ui.is_rect_visible(rect) {
        let (symbol, font) = match class {
            Some(class) => (
                class_glyph(class),
                FontId::new(22.0, FontFamily::Name("TF2 Classes".into())),
            ),
            None => ("?", FontId::proportional(20.0)),
        };
        ui.painter()
            .text(rect.center(), Align2::CENTER_CENTER, symbol, font, Color32::WHITE);
    }
    response.on_hover_text(tooltip)
}

// Codepoints from Qixalite/tf2-classfont. These render TF2 class silhouettes,
// not emoji, when drawn using the dedicated TF2 Classes font family.
fn class_glyph(class: PlayerClass) -> &'static str {
    match class {
        PlayerClass::Scout => "\u{1f407}",
        PlayerClass::Soldier => "\u{1f4a5}",
        PlayerClass::Pyro => "\u{1f525}",
        PlayerClass::Demoman => "\u{1f4a3}",
        PlayerClass::Heavy => "\u{1f4aa}",
        PlayerClass::Engineer => "\u{1f527}",
        PlayerClass::Medic => "\u{1f497}",
        PlayerClass::Sniper => "\u{1f3b7}",
        PlayerClass::Spy => "\u{1f4e6}",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tf2::lobby::PlayerKill;

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
    fn embedded_font_contains_all_nine_class_symbols() {
        use PlayerClass::*;
        let ctx = egui::Context::default();
        super::super::theme::install(&ctx);
        let _ = ctx.run(Default::default(), |ctx| {
            ctx.fonts_mut(|fonts| {
                let font = FontId::new(22.0, FontFamily::Name("TF2 Classes".into()));
                for class in [
                    Scout, Soldier, Pyro, Demoman, Heavy, Engineer, Medic, Sniper, Spy,
                ] {
                    assert!(fonts.has_glyphs(&font, class_glyph(class)), "{class:?}");
                }
            });
        });
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
