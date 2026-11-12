use super::markings::add_new_player_value;
use crate::{
    tf2::lobby::{
        AccountAge::{Approx, Loaded, Loading, Private, Unknown},
        Player,
    },
    utils::duration_as_string,
};
use eframe::egui::Ui;

pub fn add_account_age(player: &Player, ui: &mut Ui) {
    match player.account_age {
        Loading => {
            ui.spinner();
        }
        Loaded(when) => {
            add_new_player_value(
                ui,
                duration_as_string(when),
                (chrono::Local::now() - when).num_days() < 365,
            )
            .on_hover_text(format!("Account created: {}", when.format("%Y-%m-%d")));
        }
        Private => {
            ui.label("Private").on_hover_text(
                "Profile is private. Will approximate account age by looking at neighboring SteamIDs",
            );
        }
        Approx(when) => {
            add_new_player_value(
                ui,
                format!("~{}", duration_as_string(when)),
                (chrono::Local::now() - when).num_days() < 365,
            )
            .on_hover_text("Approximated by looking at neighboring SteamIDs");
        }
        Unknown => {
            ui.label("Unknown")
                .on_hover_text("Could not approximate account age");
        }
    }
}
