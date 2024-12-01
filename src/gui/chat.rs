use super::{
    colors::{hexrgb, CHAT_BLU_COLOR, CHAT_RED_COLOR},
    markings::add_flags,
    player_tooltip::add_player_tooltip,
};
use crate::{
    models::AppWin,
    tf2::lobby::{LobbyChat, LobbyFeedItem, LobbyKill, Team},
};
use eframe::egui::{text::LayoutJob, Color32, ScrollArea, TextFormat, TextStyle, Ui};

pub fn add_chat(ui: &mut Ui, app_win: &mut AppWin) {
    ui.label("Feed");

    let text_style = TextStyle::Body;
    let row_height = ui.text_style_height(&text_style);
    let num_rows = app_win.lobby.feed.len();

    ScrollArea::vertical()
        .stick_to_bottom(true)
        .auto_shrink(false)
        .show_rows(ui, row_height, num_rows, |ui, row_range| {
            ui.scope(|ui| {
                ui.style_mut().visuals.panel_fill = hexrgb(0xffffff);

                for row in row_range {
                    add_feed_row(ui, app_win, row);
                }
            });
        });
}

fn add_feed_row(ui: &mut Ui, app_win: &mut AppWin, row: usize) {
    let feed_row = &app_win.lobby.feed[row].clone();
    match feed_row {
        LobbyFeedItem::Chat(lobby_chat) => add_chat_row(ui, app_win, lobby_chat),
        LobbyFeedItem::Kill(lobby_kill) => add_kill_row(ui, app_win, lobby_kill),
    }
}

fn add_chat_row(ui: &mut Ui, app_win: &mut AppWin, chat_row: &LobbyChat) {
    ui.horizontal_wrapped(|ui| {
        let player = app_win.lobby.get_player(None, Some(chat_row.steamid));

        // In the case the player left, the name from the log file is used
        // And show an unknown team color
        let mut player_name = &chat_row.player_name;
        let mut team: Team = Team::Unknown;

        if let Some(player) = player {
            player_name = &player.name;
            team = player.team;

            add_flags(ui, player);
        }

        let color = match team {
            Team::Blue => CHAT_BLU_COLOR,
            Team::Red => CHAT_RED_COLOR,
            _ => Color32::GRAY,
        };

        let mut job = LayoutJob::default();

        job.append(
            "💬 ",
            0.0,
            TextFormat {
                color: Color32::WHITE,
                ..Default::default()
            },
        );

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
        } else if ui.label(job).clicked() {
            app_win.set_selected_player(chat_row.steamid);
        }
    });
}

fn add_kill_row(ui: &mut Ui, app_win: &mut AppWin, kill_row: &LobbyKill) {
    ui.horizontal_wrapped(|ui| {
        let killer = app_win.lobby.get_player(None, Some(kill_row.killer));
        let victim = app_win.lobby.get_player(None, Some(kill_row.victim));

        let (killer_name, killer_team) = match killer {
            Some(player) => (&player.name, player.team),
            None => (&format!("{}", &kill_row.killer.to_u64()), Team::Unknown),
        };
        let (victim_name, victim_team) = match victim {
            Some(player) => (&player.name, player.team),
            None => (&format!("{}", &kill_row.victim.to_u64()), Team::Unknown),
        };

        let killer_color = match killer_team {
            Team::Blue => CHAT_BLU_COLOR,
            Team::Red => CHAT_RED_COLOR,
            _ => Color32::GRAY,
        };
        let victim_color = match victim_team {
            Team::Blue => CHAT_BLU_COLOR,
            Team::Red => CHAT_RED_COLOR,
            _ => Color32::GRAY,
        };

        let mut job = LayoutJob::default();

        // Uniceode skulls: "☠💀🕱"

        job.append(
            "💀 ",
            0.0,
            TextFormat {
                color: Color32::WHITE,
                ..Default::default()
            },
        );

        // Killer name in team color
        job.append(
            killer_name,
            0.0,
            TextFormat {
                color: killer_color,
                ..Default::default()
            },
        );

        job.append(
            " killed ",
            0.0,
            TextFormat {
                color: Color32::LIGHT_GRAY,
                ..Default::default()
            },
        );

        // Victim name in team color
        job.append(
            victim_name,
            0.0,
            TextFormat {
                color: victim_color,
                ..Default::default()
            },
        );

        job.append(
            format!(" with {}", kill_row.weapon).as_str(),
            0.0,
            TextFormat {
                color: Color32::LIGHT_GRAY,
                ..Default::default()
            },
        );

        ui.label(job)
    });
}
