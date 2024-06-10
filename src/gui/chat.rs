use crate::{
    models::AppWin,
    tf2::lobby::{Player, Team},
};
use eframe::egui::{text::LayoutJob, Color32, ScrollArea, TextFormat, TextStyle, Ui};

use super::{colors::hexrgb, markings::add_flags};

pub fn add_chat(ui: &mut Ui, app_win: &mut AppWin) {
    let mut sorted_players: Vec<Player> = app_win.lobby.players.clone();

    if app_win.swap_team_colors {
        sorted_players.iter_mut().for_each(|p| {
            p.team = match p.team {
                Team::Invaders => Team::Defendes,
                Team::Defendes => Team::Invaders,
                x => x,
            }
        });
    }

    ui.label("Chat");
    // ui.separator();
    // ui.add_space(4.0);

    // hexrgb(0x756B5E);
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
    let chat = &app_win.lobby.chat[row].clone();
    let player = app_win.lobby.get_player(None, Some(chat.steamid));
    if player.is_none() {
        return;
    }

    let player = &player.unwrap().clone();

    ui.horizontal(|ui| {
        add_flags(ui, player);

        let mut job = LayoutJob::default();

        if chat.dead && chat.team {
            job.append(
                "*DEAD*(TEAM) ",
                0.0,
                TextFormat {
                    color: Color32::WHITE,
                    ..Default::default()
                },
            );
        } else if chat.dead {
            job.append(
                "*DEAD* ",
                0.0,
                TextFormat {
                    color: Color32::LIGHT_GRAY,
                    ..Default::default()
                },
            );
        } else if chat.team {
            job.append(
                "(TEAM) ",
                0.0,
                TextFormat {
                    color: Color32::LIGHT_GRAY,
                    ..Default::default()
                },
            );
        }
        let mut team = player.team;
        if app_win.swap_team_colors {
            team = match team {
                Team::Invaders => Team::Defendes,
                Team::Defendes => Team::Invaders,
                x => x,
            }
        }
        let color = match team {
            Team::Invaders => super::colors::CHAT_BLU_COLOR,
            Team::Defendes => super::colors::CHAT_RED_COLOR,
            _ => Color32::GRAY,
        };

        job.append(
            &player.name,
            0.0,
            TextFormat {
                color,
                ..Default::default()
            },
        );

        job.append(
            ": ",
            0.0,
            TextFormat {
                color: Color32::LIGHT_GRAY,
                ..Default::default()
            },
        );

        job.append(
            &chat.message,
            0.0,
            TextFormat {
                color: Color32::from_rgb(210, 210, 210),
                ..Default::default()
            },
        );

        if ui.label(job).clicked() {
            app_win.set_selected_player(player.steamid);
        }
    });
}
// pub fn add_chat(ui: &mut Ui, lobby: &Lobby) {
//     let text_style = TextStyle::Body;
//     let row_height = ui.text_style_height(&text_style);
//     let num_rows = lobby.chat.len();

//     ScrollArea::vertical().auto_shrink(false).show_rows(
//         ui,
//         row_height,
//         num_rows,
//         |ui, row_range| {
//             for row in row_range {
//                 let text = format!("This is row {}/{}", row + 1, num_rows);
//                 ui.label(text);
//             }
//         },
//     );

//     ui.vertical(|ui| {
//         ui.label("Chat");
//         ui.separator();
//         ui.vertical(|ui| {
//             let lines: Vec<&String> = lobby.chat.iter().rev().take(10).collect();
//             for line in lines {
//                 ui.label(line);
//             }
//         });
//     });
// }

// pub trait View {
//     fn ui(&mut self, ui: &mut egui::Ui);
// }

// #[derive(Default, PartialEq)]
// struct ScrollStickTo {
//     n_items: usize,
// }

// impl View for ScrollStickTo {
//     fn ui(&mut self, ui: &mut Ui) {
//         ui.label("Rows enter from the bottom, we want the scroll handle to start and stay at bottom unless moved");

//         ui.add_space(4.0);

//         let text_style = TextStyle::Body;
//         let row_height = ui.text_style_height(&text_style);
//         ScrollArea::vertical().stick_to_bottom(true).show_rows(
//             ui,
//             row_height,
//             self.n_items,
//             |ui, row_range| {
//                 for row in row_range {
//                     let text = format!("This is row {}", row + 1);
//                     ui.label(text);
//                 }
//             },
//         );

//         self.n_items += 1;
//         ui.ctx().request_repaint();
//     }
// }
