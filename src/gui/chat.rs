use super::{
    colors::{hexrgb, CHAT_BLU_COLOR, CHAT_RED_COLOR},
    markings::add_flags,
    player_tooltip::add_player_tooltip,
};
use crate::{models::AppWin, tf2::lobby::Team};
use eframe::egui::{text::LayoutJob, Color32, ScrollArea, TextFormat, TextStyle, Ui};

pub fn add_chat(ui: &mut Ui, app_win: &mut AppWin) {
    ui.label("Chat");

    let text_style = TextStyle::Body;
    let row_height = ui.text_style_height(&text_style);
    let num_rows = app_win.lobby.chat.len();

    ScrollArea::vertical()
        .stick_to_bottom(true)
        .auto_shrink(false)
        .show_rows(ui, row_height, num_rows, |ui, row_range| {
            ui.scope(|ui| {
                ui.style_mut().visuals.panel_fill = hexrgb(0xffffff);

                for row in row_range {
                    add_chat_row(ui, app_win, row);
                }
            });
        });
}

fn add_chat_row(ui: &mut Ui, app_win: &mut AppWin, row: usize) {
    ui.horizontal(|ui| {
        let chat_row = &app_win.lobby.chat[row];
        let player = app_win.lobby.get_player(None, Some(chat_row.steamid));

        // In the case the player left, the name from the log file is used
        // And show an unknown team color
        let mut player_name = &chat_row.player_name;
        let mut team: Team = Team::Unknown;

        if let Some(player) = player {
            player_name = &player.name;
            team = player.team;

            if app_win.swap_team_colors {
                team = match player.team {
                    Team::Invaders => Team::Defendes,
                    Team::Defendes => Team::Invaders,
                    x => x,
                }
            }

            add_flags(ui, player);
        }

        let color = match team {
            Team::Invaders => CHAT_BLU_COLOR,
            Team::Defendes => CHAT_RED_COLOR,
            _ => Color32::GRAY,
        };

        let mut job = LayoutJob::default();

        // Prefix player name with *DEAD* and (TEAM) if needed
        if chat_row.dead {
            job.append(
                "*DEAD* ",
                0.0,
                TextFormat {
                    color: Color32::WHITE,
                    ..Default::default()
                },
            );
        }

        if chat_row.team {
            job.append(
                "(TEAM) ",
                0.0,
                TextFormat {
                    color: Color32::WHITE,
                    ..Default::default()
                },
            );
        }

        // Player name
        job.append(
            player_name,
            0.0,
            TextFormat {
                color,
                ..Default::default()
            },
        );

        // The : between player name and the chat message
        job.append(
            ": ",
            0.0,
            TextFormat {
                color: Color32::LIGHT_GRAY,
                ..Default::default()
            },
        );

        // The chat message
        job.append(
            &chat_row.message,
            0.0,
            TextFormat {
                color: Color32::from_rgb(210, 210, 210),
                ..Default::default()
            },
        );

        // Add the formatted text to the UI and make it clickable
        if let Some(player) = player {
            if ui
                .label(job)
                .on_hover_ui_at_pointer(|ui| add_player_tooltip(ui, player))
                .clicked()
            {
                app_win.set_selected_player(chat_row.steamid);
            }
        } else {
            if ui.label(job).clicked() {
                app_win.set_selected_player(chat_row.steamid);
            }
        }
    });
}
