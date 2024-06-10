use super::{
    colors::{TEAM_BLU_COLOR, TEAM_RED_COLOR},
    markings::add_flags,
    player_tooltip::add_player_tooltip,
};
use crate::{
    models::{steamid::SteamID, AppWin},
    tf2::lobby::{Player, Team},
};
use eframe::egui::{Align, Color32, Grid, Layout, Sense, Ui, Vec2};

pub fn scoreboard_team(
    app_win: &mut AppWin,
    ui: &mut Ui,
    title: &str,
    players: &Vec<&Player>,
    team_name: &str,
) {
    ui.heading(format!("{} - {} players", title, players.len()));

    ui.horizontal(|ui| {
        let total_kills = players.iter().map(|p| p.kills).sum::<u32>();
        let total_crit_kills = players.iter().map(|p| p.crit_kills).sum::<u32>();
        let total_deaths = players.iter().map(|p| p.deaths).sum::<u32>();
        let total_crits_deaths = players.iter().map(|p| p.crit_deaths).sum::<u32>();

        ui.label(format!("Kills: {}", total_kills));
        if app_win.show_crits {
            ui.colored_label(Color32::GRAY, format!("({})", total_crit_kills));
        }

        ui.label(format!("Deaths: {}", total_deaths));
        if app_win.show_crits {
            ui.colored_label(Color32::GRAY, format!("({})", total_crits_deaths));
        }
    });

    Grid::new(team_name).striped(true).show(ui, |ui| {
        ui.with_layout(Layout::top_down(Align::RIGHT), |ui| {
            ui.label("");
        });
        ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
            // ui.label(RichText::new("Player").strong());
            ui.label("Player");
        });
        ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
            // ui.label(RichText::new("Kills").strong());
            ui.label("Kills");
        });
        ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
            // ui.label(RichText::new("Deaths").strong());
            ui.label("Deaths");
        });
        // ui.label("Flags");
        ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
            // ui.label(RichText::new("Links").strong());
            ui.label("Flags");
        });

        ui.end_row();

        for player in players {
            // Team color box
            add_team_symbol(app_win, ui, app_win.self_steamid, player);

            add_player_name(app_win, ui, player);

            // Player kills
            ui.horizontal(|ui| {
                ui.label(format!("{:3}", player.kills))
                    .on_hover_text("Number of kills");
                if app_win.show_crits {
                    ui.colored_label(Color32::GRAY, format!("({})", player.crit_kills))
                        .on_hover_text("Number of crit kills");
                }
            });

            // Player deaths
            ui.horizontal(|ui| {
                ui.label(format!("{:3}", player.deaths))
                    .on_hover_text("Number of deaths");

                if app_win.show_crits {
                    ui.colored_label(Color32::GRAY, format!("({})", player.crit_deaths))
                        .on_hover_text("Number of deaths due to crits");
                }
            });

            add_flags(ui, player);

            ui.end_row();
        }

        // Add empty rows to fill the grid
        if players.len() < 12 {
            for _ in 0..(12 - players.len()) {
                ui.label("");
                ui.label("");
                ui.label("");
                ui.label("");
                ui.end_row();
            }
        }
    });
}

fn add_player_name(app_win: &mut AppWin, ui: &mut Ui, player: &Player) {
    // Player icon and name
    ui.horizontal(|ui| {
        ui.push_id(player.steamid.to_u64(), |ui| {
            ui.scope(|ui| {
                // let color = match player.team {
                //     Team::Invaders => TEAM_BLU_COLOR,
                //     Team::Defendes => TEAM_RED_COLOR,
                //     _ => Color32::GRAY,
                // };
                // let color = Color32::WHITE;

                // ui.visuals_mut().override_text_color = Some(color);
                // ui.style_mut().visuals.override_text_color = Some(color);

                // Mark selected player and friends of selected player
                if let Some(steamid) = app_win.selected_player {
                    if steamid == player.steamid
                        || app_win
                            .lobby
                            .friendships
                            .are_friends(player.steamid, steamid)
                    {
                        let marked_color = Some(Color32::YELLOW);
                        ui.visuals_mut().override_text_color = marked_color;
                        ui.style_mut().visuals.override_text_color = marked_color;
                    }
                }

                // Player avatar and hover tooltip
                if let Some(steam_info) = &player.steam_info {
                    ui.image(&steam_info.avatar)
                        .on_hover_ui_at_pointer(|ui| add_player_tooltip(ui, player));
                }

                // Player name and hover tooltip
                if ui
                    .label(&player.name)
                    .on_hover_ui_at_pointer(|ui| add_player_tooltip(ui, player))
                    .clicked()
                {
                    app_win.set_selected_player(player.steamid);
                }
            });
        });
    });
}

fn add_team_symbol(app_win: &mut AppWin, ui: &mut Ui, self_steamid: SteamID, player: &Player) {
    let color = match player.team {
        Team::Invaders => TEAM_BLU_COLOR,
        Team::Defendes => TEAM_RED_COLOR,
        _ => Color32::GRAY,
    };

    ui.with_layout(Layout::top_down(Align::RIGHT), |ui| {
        ui.horizontal(|ui| {
            let size = Vec2::splat(2.0 * 10.0 * 0.5 + 5.0);

            let (rect, response) = ui.allocate_at_least(size, Sense::hover());
            ui.painter().rect_filled(rect, 3.0f32, color);
            response.on_hover_text("Team color");

            let pos = rect.center();
            app_win.friendship_positions.insert(player.steamid, pos);

            if player.steamid == self_steamid {
                let (rect, response) = ui.allocate_at_least(size, Sense::hover());
                ui.painter().rect_filled(rect, 3.0f32, Color32::WHITE);
                response.on_hover_text("This is you");
            }

            if let Some(tooltip) = &player.is_newbie() {
                let (rect, response) = ui.allocate_at_least(size, Sense::hover());
                ui.painter().rect_filled(rect, 3.0f32, Color32::GREEN);
                response.on_hover_text(tooltip);
            }

            if let Some(tooltip) = &player.has_steam_bans() {
                let (rect, response) = ui.allocate_at_least(size, Sense::hover());
                ui.painter().rect_filled(rect, 3.0f32, Color32::RED);
                response.on_hover_text(tooltip);
            }

            if app_win
                .lobby
                .friendships
                .are_friends(self_steamid, player.steamid)
            {
                ui.colored_label(Color32::RED, "❤")
                    .on_hover_text(format!("{} is in your friendlist", player.name));
            }
        });
    });
}
