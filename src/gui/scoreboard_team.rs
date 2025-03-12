use super::{
    account_age::add_account_age,
    colors::{TEAM_BLU_COLOR, TEAM_RED_COLOR},
    markings::add_flags,
    player_tooltip::add_player_tooltip,
    playtime::add_playtime,
};
use crate::{
    models::{steamid::SteamID, AppWin},
    tf2::lobby::{Player, Team},
};
use eframe::egui::{
    text::LayoutJob, Align, Color32, CursorIcon, Grid, Layout, Sense, TextFormat, Ui, Vec2,
};

/// This is draws a scoreboard for a single team
pub fn scoreboard_team(app_win: &mut AppWin, ui: &mut Ui, title: &str, players: &Vec<&Player>) {
    ui.heading(format!("{} - {} players", title, players.len()));

    ui.horizontal(|ui| {
        let total_kills = players.iter().map(|p| p.kills).sum::<u32>();
        let total_crit_kills = players.iter().map(|p| p.crit_kills).sum::<u32>();
        let total_deaths = players.iter().map(|p| p.deaths).sum::<u32>();
        let total_crits_deaths = players.iter().map(|p| p.crit_deaths).sum::<u32>();

        ui.label(format!("Kills: {}", total_kills));
        if app_win.app_settings.show_crits {
            ui.colored_label(Color32::GRAY, format!("({})", total_crit_kills));
        }

        ui.label(format!("Deaths: {}", total_deaths));
        if app_win.app_settings.show_crits {
            ui.colored_label(Color32::GRAY, format!("({})", total_crits_deaths));
        }
    });

    Grid::new(title).striped(true).show(ui, |ui| {
        ui.with_layout(Layout::top_down(Align::RIGHT), |ui| {
            ui.label("");
        });
        ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
            // ui.label(RichText::new("Player").strong());
            ui.label("Player");
        });
        // ui.with_layout(Layout::top_down(Align::RIGHT), |ui| {
        //     // ui.label(RichText::new("Kills").strong());
        //     ui.label("Score");
        // });
        ui.with_layout(Layout::top_down(Align::RIGHT), |ui| {
            // ui.label(RichText::new("Kills").strong());
            ui.label("Kills");
        });
        ui.with_layout(Layout::top_down(Align::RIGHT), |ui| {
            // ui.label(RichText::new("Deaths").strong());
            ui.label("Deaths");
        });
        ui.with_layout(Layout::top_down(Align::RIGHT), |ui| {
            // ui.label(RichText::new("Deaths").strong());
            ui.label("Age").on_hover_text("Account age");
        });
        ui.with_layout(Layout::top_down(Align::RIGHT), |ui| {
            // ui.label(RichText::new("Deaths").strong());
            ui.label("Hours").on_hover_text("TF2 hours played");
        });
        // ui.label("Flags");
        ui.with_layout(Layout::top_down(Align::RIGHT), |ui| {
            // ui.label(RichText::new("Deaths").strong());
            ui.label("Ping");
        });
        // ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
        //     // ui.label(RichText::new("Links").strong());
        //     ui.label("Last Kill");
        // });
        ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
            // ui.label(RichText::new("Links").strong());
            ui.label("Flags");
        });

        ui.end_row();

        for player in players {
            // Team color box
            add_team_symbol(app_win, ui, app_win.self_steamid, player);

            add_player_name(app_win, ui, player);

            // Player score
            // ui.with_layout(Layout::top_down(Align::RIGHT), |ui| {
            //     ui.label(format!("{:3}", player.score))
            //         .on_hover_text("Score");
            // });

            // Player kills
            ui.with_layout(Layout::top_down(Align::RIGHT), |ui| {
                // ui.label(format!("{:3}", player.kills))
                //     .on_hover_text("Number of kills (crit kills)");

                let mut job = LayoutJob {
                    break_on_newline: false,
                    ..Default::default()
                };

                job.append(
                    format!("{:3}", player.kills).as_str(),
                    0.0,
                    TextFormat {
                        color: ui.style().visuals.text_color(),
                        ..Default::default()
                    },
                );

                if app_win.app_settings.show_crits {
                    job.append(
                        format!("({})", player.crit_kills).as_str(),
                        6.0,
                        TextFormat {
                            color: Color32::GRAY,
                            ..Default::default()
                        },
                    );
                }
                ui.label(job).on_hover_text("Number of kills (crit kills)");
            });

            // Player deaths
            ui.with_layout(Layout::top_down(Align::RIGHT), |ui| {
                let mut job = LayoutJob::default();

                job.append(
                    format!("{}", player.deaths).as_str(),
                    0.0,
                    TextFormat {
                        color: ui.style().visuals.text_color(),
                        ..Default::default()
                    },
                );

                if app_win.app_settings.show_crits {
                    job.append(
                        format!(" ({})", player.crit_deaths).as_str(),
                        0.0,
                        TextFormat {
                            color: Color32::GRAY,
                            ..Default::default()
                        },
                    );
                }
                ui.label(job)
                    .on_hover_text("Number of deaths (crit deaths)");
            });

            ui.with_layout(Layout::top_down(Align::RIGHT), |ui| {
                add_account_age(player, ui);
            });

            ui.with_layout(Layout::top_down(Align::RIGHT), |ui| {
                add_playtime(ui, player);
            });

            ui.with_layout(Layout::top_down(Align::RIGHT), |ui| {
                ui.label(format!("{}", player.pingms))
                    .on_hover_text_at_pointer("ms");
            });

            // if let Some(k) = player.kills_with.last() {
            //     let s = format!("{}{}", k.weapon, if k.crit { " (crit)" } else { "" });
            //     ui.label(s);
            // } else {
            //     ui.label("");
            // }

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
                ui.label("");
                ui.label("");
                ui.label("");
                ui.label("");
                // ui.label("");
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
                    // if player.steamid.to_u64() != 76561198850780330 {
                    //     log::info!("Player avatar: {}", steam_info.avatar);
                    ui.image(&steam_info.avatar)
                        .on_hover_cursor(CursorIcon::Help)
                        .on_hover_ui_at_pointer(|ui| add_player_tooltip(ui, player));
                    // }
                }

                // Player name prefixed with DEAD is not alive
                let name_color = if player.alive {
                    Color32::WHITE
                } else {
                    Color32::GRAY
                };

                // Player name and hover tooltip
                if ui
                    .colored_label(name_color, player.name.clone())
                    .on_hover_cursor(CursorIcon::Default)
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
        Team::Blue => TEAM_BLU_COLOR,
        Team::Red => TEAM_RED_COLOR,
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
