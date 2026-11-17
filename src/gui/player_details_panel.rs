use super::{
    colors::hex_to_rgb, comments::add_profile_comments, player_flag_editor::add_player_flag_editor,
    playtime::add_playtime,
};
use crate::{
    models::{AppWin, steamid::SteamID},
    tf2::lobby::{Player, PlayerKill},
};
use eframe::egui::{self, Image, OpenUrl, ScrollArea, Ui, Vec2};

pub fn add_player_details_panel(app_win: &mut AppWin, ui: &mut Ui) {
    ui.heading("Player Details");

    let steamid = app_win.selected_player;
    if steamid.is_none() {
        ui.label("Select a player to see their details.");
        return;
    }

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            add_player_details_for_steamid(app_win, ui, steamid.unwrap());
        });
}

pub fn show_player_details_windows(app_win: &mut AppWin, ctx: &egui::Context) {
    if app_win.open_player_details_windows.is_empty() {
        return;
    }

    let open_windows = app_win.open_player_details_windows.clone();
    let mut still_open = Vec::with_capacity(open_windows.len());

    for steamid in open_windows {
        let mut window_open = true;
        let window_title = player_window_title(app_win, steamid);

        super::theme::window(ctx, window_title)
            .id(egui::Id::unique((
                "player_details_window",
                steamid.to_u64(),
            )))
            .open(&mut window_open)
            .resizable(true)
            .default_width(420.0)
            .default_height(640.0)
            .show(ctx, |ui| {
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        add_player_details_for_steamid(app_win, ui, steamid);
                    });
            });

        if window_open {
            still_open.push(steamid);
        }
    }

    app_win.open_player_details_windows = still_open;
}

pub fn add_player_details_for_steamid(app_win: &AppWin, ui: &mut Ui, steamid: SteamID) {
    let profile = match app_win.player_profile(steamid) {
        Ok(profile) => profile,
        Err(error) => {
            ui.colored_label(
                ui.visuals().error_fg_color,
                format!("Could not load player: {error}"),
            );
            return;
        }
    };
    let live = app_win.lobby.get_player(None, Some(steamid));
    let mut display = profile.display_player();
    if profile.steam_info.is_none()
        && let Some(live) = live
    {
        display.name = live.name.clone();
    }
    display.reputation = app_win
        .profile_reputations
        .get(&steamid)
        .cloned()
        .or_else(|| live.and_then(|p| p.reputation.clone()));
    let player = &display;
    let status = app_win.profile_status.get(&steamid);
    let loading =
        status.is_none() || status == Some(&crate::models::player_profile::FetchStatus::Loading);
    let missing = if loading {
        "Loading…"
    } else if status == Some(&crate::models::player_profile::FetchStatus::Failed) {
        "Could not fetch data"
    } else {
        "Not available"
    };
    if let Some(live) = live {
        add_player_kick_buttons(app_win, live, ui);
    }
    ui.add_space(8.0);

    ui.horizontal(|ui| {
        if player.steam_info.is_some() {
            add_player_avatar(player, ui);
        } else {
            ui.label(missing);
        }

        ui.vertical(|ui| {
            ui.heading(player.name.as_str());

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

            if player.tf2_play_minutes == crate::tf2::lobby::Tf2PlayMinutes::Loading && !loading {
                ui.label(format!("TF2 playtime: {missing}"));
            } else {
                add_playtime(ui, player);
            }

            if let Some(friends) = &player.friends {
                ui.label(format!("{} friends", friends.len()));
            } else {
                ui.label(format!("Friends: {missing}"));
            }

            if let Some(reason) = player.has_steam_bans() {
                ui.label(reason);
            } else if player.steam_bans.is_none() {
                ui.label(format!("Steam bans: {missing}"));
            } else {
                ui.label("No Steam bans");
            }
        });
    });

    ui.label("");

    add_player_community_links(player, ui);

    ui.label("");

    add_player_flag_editor(app_win, ui, player);

    ui.add_space(12.0);
    super::player_notes::show(app_win, ui, steamid);

    ui.label("");

    add_player_sourcebans(player, ui);

    ui.label("");

    if let Some(session) = app_win
        .match_snapshots
        .get(&steamid)
        .filter(|s| s.session_id == app_win.lobby.session_id)
    {
        ui.heading(if live.is_some() {
            "Current session"
        } else {
            "Last seen in this session"
        });
        ui.label(format!(
            "Kills: {}  Deaths: {}",
            session.kills, session.deaths
        ));
        if live.is_some() {
            ui.label(format!(
                "Team: {:?}  Ping: {} ms",
                session.team, session.ping_ms
            ));
            if let Some(health) = session.health {
                ui.label(format!("Health: {health}"));
            }
        }
        add_player_kills(&session.kills_with, ui);
    } else {
        ui.label("Not on the current server");
    }

    ui.label("");

    if player.profile_comments.is_none() && !loading {
        ui.heading("Profile Comments");
        ui.label(missing);
    } else {
        add_profile_comments(player, ui);
    }
}

fn player_window_title(app_win: &AppWin, steamid: SteamID) -> String {
    let name = app_win
        .player_profile(steamid)
        .map(|p| p.name)
        .unwrap_or_else(|_| steamid.to_u64().to_string());
    format!("Player Details - {name}")
}

fn add_player_avatar(player: &Player, ui: &mut Ui) {
    if let Some(steam_info) = &player.steam_info {
        let image = Image::from_uri(&steam_info.avatar_full)
            .fit_to_exact_size(Vec2::new(100.0, 100.0))
            .corner_radius(3.0);

        super::theme::image(ui, image);
    } else {
        ui.label("Loading...");
    }
}

fn add_player_kills(kills_with: &[PlayerKill], ui: &mut Ui) {
    ui.heading("Kills");

    if kills_with.is_empty() {
        ui.label("No kills yet");
        return;
    }

    let kills: Vec<&PlayerKill> = kills_with.iter().rev().take(20).collect();

    let names = kills
        .iter()
        .map(|k| {
            format!(
                "{}{}",
                k.weapon.replace('_', " "),
                if k.crit { " (crit)" } else { "" }
            )
        })
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

    ui.heading("More Info:");
    ui.horizontal(|ui| {
        ui.scope(|ui| {
            ui.style_mut().visuals.widgets.inactive.weak_bg_fill = hex_to_rgb(0x89661D);

            // ui.label("View on");
            make_link(ui, player.steamid.steam_community_url(), "Steam");
            make_link(ui, player.steamid.steam_history_url(), "SteamHistory");
            // make_link(ui, player.steamid.steam_rep_url(), "SteamRep");
            // make_link(ui, player.steamid.steam_id_url(), "SteamID");
            make_link(ui, player.steamid.rep_tf_url(), "Rep.TF");
        });
    });
}

fn add_player_kick_buttons(app_win: &AppWin, player: &Player, ui: &mut Ui) {
    ui.heading("Kick Player:");
    ui.horizontal_wrapped(|ui| {
        ui.scope(|ui| {
            ui.style_mut().visuals.widgets.inactive.weak_bg_fill = hex_to_rgb(0x89161D);
            ui.style_mut().visuals.widgets.hovered.weak_bg_fill = hex_to_rgb(0x631016);

            if ui.button("Kick for Cheating").clicked() {
                log::info!("Voting to kick player '{}' for cheating", player.name);
                let cmd = format!("callvote kick \"{} cheating\"", player.id);
                let current = app_win.shared_lobby.get();
                if current.session_id == app_win.lobby.session_id
                    && current
                        .get_player(None, Some(player.steamid))
                        .is_some_and(|p| p.id == player.id)
                {
                    app_win.bus.lock().unwrap().send_rcon_cmd(cmd.as_str());
                }
            }
        });

        ui.scope(|ui| {
            ui.style_mut().visuals.widgets.inactive.weak_bg_fill = hex_to_rgb(0x89161D);
            ui.style_mut().visuals.widgets.hovered.weak_bg_fill = hex_to_rgb(0x631016);

            if ui.button("Kick for Idle").clicked() {
                log::info!("Voting to kick player '{}' for idling", player.name);
                let cmd = format!("callvote kick \"{} idle\"", player.id);
                let current = app_win.shared_lobby.get();
                if current.session_id == app_win.lobby.session_id
                    && current
                        .get_player(None, Some(player.steamid))
                        .is_some_and(|p| p.id == player.id)
                {
                    app_win.bus.lock().unwrap().send_rcon_cmd(cmd.as_str());
                }
            }
        });

        ui.scope(|ui| {
            ui.style_mut().visuals.widgets.inactive.weak_bg_fill = hex_to_rgb(0x89161D);
            ui.style_mut().visuals.widgets.hovered.weak_bg_fill = hex_to_rgb(0x631016);

            if ui.button("Kick for Scamming").clicked() {
                log::info!("Voting to kick player '{}' for scamming", player.name);
                let cmd = format!("callvote kick \"{} scamming\"", player.id);
                let current = app_win.shared_lobby.get();
                if current.session_id == app_win.lobby.session_id
                    && current
                        .get_player(None, Some(player.steamid))
                        .is_some_and(|p| p.id == player.id)
                {
                    app_win.bus.lock().unwrap().send_rcon_cmd(cmd.as_str());
                }
            }
        });
    });
}

fn add_player_sourcebans(player: &Player, ui: &mut Ui) {
    ui.heading("SourceBans");
    let Some(reputation) = &player.reputation else {
        ui.label("Community ban data is not available yet");
        return;
    };
    if reputation.source_bans.is_empty() {
        ui.label("No bans found in available data");
    }
    let today = chrono::Utc::now().date_naive();
    for ban in &reputation.source_bans {
        super::markings::add_sourceban_row(ui, ban, today);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::player_profile::{FetchStatus, seed_account, test_app},
        tf2::lobby::Player,
    };

    fn render(app: &AppWin, id: SteamID) -> String {
        let ctx = egui::Context::default();
        super::super::theme::install(&ctx);
        let output = super::super::tests::run_ui(
            &ctx,
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(650.0, 1600.0),
                )),
                ..Default::default()
            },
            |root| {
                egui::CentralPanel::default()
                    .show(root, |ui| add_player_details_for_steamid(app, ui, id));
            },
        );
        output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn database_player_has_profile_flags_and_notes_without_lobby_membership() {
        let mut app = test_app();
        let id = SteamID::from_u64(crate::models::steamid::MIN_STEAMID64 + 1);
        seed_account(&app, id, "Offline account");
        app.profile_status.insert(id, FetchStatus::Unavailable);
        crate::db::queries::save_player_note(
            &mut app.db.get().unwrap(),
            id.to_u64() as i64,
            "Reason for flag",
        )
        .unwrap();
        let text = render(&app, id);
        assert!(text.contains("Offline account"));
        assert!(text.contains("Flags:"));
        assert!(text.contains("Notes"));
        assert!(text.contains("Reason for flag"));
        assert!(text.contains("Not on the current server"));
        assert!(!text.contains("Kick Player"));
        assert!(!text.contains("not found"));
        assert!(!text.contains("Loading friends"));
    }

    #[test]
    fn kick_controls_require_current_membership_and_stats_do_not_cross_sessions() {
        let mut app = test_app();
        let id = SteamID::from_u64(crate::models::steamid::MIN_STEAMID64 + 1);
        seed_account(&app, id, "Player");
        let mut lobby = app.shared_lobby.get();
        lobby.players.push(Player {
            steamid: id,
            id: 7,
            kills: 9,
            deaths: 2,
            ..Default::default()
        });
        app.shared_lobby.set(lobby.clone());
        app.get_latest_lobby();
        let text = render(&app, id);
        assert!(text.contains("Kick Player"));
        assert!(text.contains("Kills: 9  Deaths: 2"));
        lobby.players.clear();
        app.shared_lobby.set(lobby.clone());
        app.get_latest_lobby();
        let text = render(&app, id);
        assert!(!text.contains("Kick Player"));
        assert!(text.contains("Last seen in this session"));
        assert!(text.contains("Kills: 9  Deaths: 2"));
        lobby.session_id += 1;
        app.shared_lobby.set(lobby);
        app.get_latest_lobby();
        let text = render(&app, id);
        assert!(!text.contains("Kills: 9"));
        assert!(text.contains("Not on the current server"));
    }
}
