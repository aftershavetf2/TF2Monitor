use crate::{tf2::lobby::Player, tf2bd::player_lists::PlayerLists};
use eframe::egui::{Image, Ui};

use super::{markings, playtime::add_playtime};

pub fn add_player_tooltip(ui: &mut Ui, player: &Player, lists: &PlayerLists) {
    ui.heading(format!("{} ({})", player.name, player.id));

    // ui.heading(format!("({}) {}", player.id, &player.name));

    if let Some(steam_info) = &player.steam_info {
        let image = Image::from_uri(&steam_info.avatar_full)
            .max_width(100.0)
            .corner_radius(3.0);

        super::theme::image(ui, image);

        ui.label(format!(
            "Account created: {}",
            steam_info.get_account_created()
        ));
    }

    add_playtime(ui, player);

    ui.label("");

    if let Some(friends) = &player.friends {
        ui.label(format!("{} friends", friends.len()));
    } else {
        ui.label("Loading friends...");
    }

    if let Some(reason) = player.has_steam_bans() {
        ui.label(reason);
    } else {
        ui.label("No Steam bans");
    }

    ui.horizontal_wrapped(|ui| {
        markings::add_flags(ui, player);
        markings::add_tf2bd_badge(ui, player.steamid, lists);
    });
    if lists.has_match(player.steamid) {
        markings::add_tf2bd_sources(ui, player.steamid, lists);
    }
}
