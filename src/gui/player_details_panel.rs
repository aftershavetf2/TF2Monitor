use super::{
    colors::hexrgb, comments::add_profile_comments, player_flag_editor::add_player_flag_editor,
    playtime::add_playtime,
};
use crate::{
    models::AppWin,
    tf2::lobby::{Player, PlayerKill},
};
use eframe::egui::{text::LayoutJob, Color32, Image, OpenUrl, TextFormat, Ui, Vec2};

pub fn add_player_details_panel(app_win: &mut AppWin, ui: &mut Ui) {
    ui.heading("Player Details");

    let steamid = app_win.selected_player;
    if steamid.is_none() {
        ui.label("Select a player to see their details.");
        return;
    }

    let steamid = steamid.unwrap();
    let player = app_win.lobby.get_player(None, Some(steamid)).or_else(|| {
        app_win
            .lobby
            .recently_left_players
            .iter()
            .find(|p| p.steamid == steamid)
    });

    if player.is_none() {
        ui.label(format!(
            "Player with SteamID {} not found.",
            steamid.to_u64()
        ));
        return;
    }
    let player = player.unwrap();

    add_player_community_links(player, ui);

    ui.horizontal(|ui| {
        add_player_avatar(player, ui);

        ui.vertical(|ui| {
            ui.heading(format!("{} ({})", player.name, player.id));

            if let Some(steam_info) = &player.steam_info {
                if steam_info.public_profile {
                    ui.label("Public profile");
                } else {
                    ui.label("Private profile");
                }

                ui.label(format!(
                    "Account created: {}",
                    steam_info.get_account_created()
                ));
            }

            ui.label(format!("SteamID64: {}", player.steamid.to_u64()));
            ui.label(format!("SteamID32: {}", player.steamid.to_steam_id32()));

            add_playtime(ui, player);

            if let Some(friends) = &player.friends {
                ui.label(format!("{} friends", friends.len()));
            } else {
                ui.label("Loading friends...");
            }

            if let Some(reason) = player.has_steam_bans() {
                ui.label(reason);
            } else if player.steam_info.is_none() {
                ui.label("Loading Steam bans...");
            } else {
                ui.label("No Steam bans");
            }
        });
    });

    ui.label("");

    add_player_kick_buttons(app_win, player, ui);

    ui.label("");

    add_player_flag_editor(app_win, ui, player);

    ui.label("");

    add_player_sourcebans(player, ui);

    ui.label("");

    add_player_kills(player, ui);

    ui.label("");

    add_profile_comments(player, ui);
}

fn add_player_avatar(player: &Player, ui: &mut Ui) {
    if let Some(steam_info) = &player.steam_info {
        let image = Image::from_uri(&steam_info.avatarfull)
            .fit_to_exact_size(Vec2::new(100.0, 100.0))
            .corner_radius(3.0);

        ui.add(image);
    } else {
        ui.label("Loading...");
    }
}

fn add_player_kills(player: &Player, ui: &mut Ui) {
    ui.heading("Kills");

    if player.kills_with.is_empty() {
        ui.label("No kills yet");
        return;
    }

    let kills: Vec<&PlayerKill> = player.kills_with.iter().rev().take(20).collect();

    let names = kills
        .iter()
        .map(|k| format!("{}{}", k.weapon, if k.crit { " (crit)" } else { "" }))
        .collect::<Vec<String>>()
        .join(", ");

    ui.label(names);
}

fn add_player_community_links(player: &Player, ui: &mut Ui) {
    fn make_link(ui: &mut Ui, url: String, text: &str) {
        if ui.button(text).clicked() {
            ui.ctx().open_url(OpenUrl {
                url: url.clone(),
                new_tab: true,
            });
        }
    }

    // ui.heading("More info:");
    ui.horizontal(|ui| {
        // ui.label("View on");
        make_link(ui, player.steamid.steam_community_url(), "Steam");
        make_link(ui, player.steamid.steam_history_url(), "SteamHistory");
        make_link(ui, player.steamid.steam_rep_url(), "SteamRep");
        make_link(ui, player.steamid.steam_id_url(), "SteamID");
        make_link(ui, player.steamid.rep_tf_url(), "Rep.TF");
    });
}

fn add_player_kick_buttons(app_win: &AppWin, player: &Player, ui: &mut Ui) {
    ui.horizontal_wrapped(|ui| {
        ui.scope(|ui| {
            ui.style_mut().visuals.widgets.inactive.weak_bg_fill = hexrgb(0x89161D);
            ui.style_mut().visuals.widgets.hovered.weak_bg_fill = hexrgb(0x631016);

            if ui.button("Kick for Cheating").clicked() {
                log::info!("Voting to kick player '{}' for cheating", player.name);
                let cmd = format!("callvote kick \"{} cheating\"", player.id);
                app_win.bus.lock().unwrap().send_rcon_cmd(cmd.as_str());
            }
        });

        ui.scope(|ui| {
            ui.style_mut().visuals.widgets.inactive.weak_bg_fill = hexrgb(0x89161D);
            ui.style_mut().visuals.widgets.hovered.weak_bg_fill = hexrgb(0x631016);

            if ui.button("Kick for Idle").clicked() {
                log::info!("Voting to kick player '{}' for idling", player.name);
                let cmd = format!("callvote kick \"{} idle\"", player.id);
                app_win.bus.lock().unwrap().send_rcon_cmd(cmd.as_str());
            }
        });

        ui.scope(|ui| {
            ui.style_mut().visuals.widgets.inactive.weak_bg_fill = hexrgb(0x89161D);
            ui.style_mut().visuals.widgets.hovered.weak_bg_fill = hexrgb(0x631016);

            if ui.button("Kick for Scamming").clicked() {
                log::info!("Voting to kick player '{}' for scamming", player.name);
                let cmd = format!("callvote kick \"{} scamming\"", player.id);
                app_win.bus.lock().unwrap().send_rcon_cmd(cmd.as_str());
            }
        });
    });
}

fn add_player_sourcebans(player: &Player, ui: &mut Ui) {
    let mut has_bans = false;

    ui.heading("SourceBans");

    if let Some(reputation) = &player.reputation {
        if reputation.has_bad_reputation && !reputation.bans.is_empty() {
            // Sort the bans by when descending, but treat the date as a string
            let mut bans = reputation.bans.clone();
            bans.sort_by_key(|ban| ban.source.clone());

            for ban in &bans {
                let mut job = LayoutJob::default();

                job.append(
                    "- ",
                    10.0,
                    TextFormat {
                        color: Color32::WHITE,
                        ..Default::default()
                    },
                );
                job.append(
                    &ban.source,
                    5.0,
                    TextFormat {
                        color: Color32::GRAY,
                        ..Default::default()
                    },
                );
                job.append(
                    format!("\"{}\"", ban.reason).as_str(),
                    5.0,
                    TextFormat {
                        color: Color32::WHITE,
                        ..Default::default()
                    },
                );

                ui.label(job);

                has_bans = true;
            }
        }
    }

    if !has_bans {
        if player.reputation.is_none() {
            ui.label("Loading SourceBans...");
        } else {
            ui.label("No bans found");
        }
    }
}
