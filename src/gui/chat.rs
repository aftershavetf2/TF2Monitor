use super::{
    colors::{CHAT_BLU_COLOR, CHAT_RED_COLOR},
    markings::add_flags,
};
use crate::{
    models::AppWin,
    tf2::lobby::{LobbyChat, Team},
};
use eframe::egui::{Color32, ScrollArea, TextFormat, TextStyle, Ui, text::LayoutJob};

pub fn add_chat(ui: &mut Ui, app_win: &mut AppWin) {
    ui.heading("Chat");

    ui.push_id("chat", |ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        ui.spacing_mut().interact_size.y = ui.text_style_height(&TextStyle::Body);
        ui.spacing_mut().button_padding.y = 1.0;

        // Entries can wrap or contain badges, so use their actual heights.
        ScrollArea::vertical()
            .stick_to_bottom(true)
            .auto_shrink(false)
            .show(ui, |ui| {
                for row in &app_win.lobby.chat.clone() {
                    add_chat_row(ui, app_win, row);
                }
            });
    });
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
            _ => super::theme::muted_text(),
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

        job.append(
            &chat_row.message,
            0.0,
            TextFormat {
                color: Color32::from_rgb(210, 210, 210),
                ..Default::default()
            },
        );

        let is_translated = chat_row.translated_message.is_some()
            && chat_row.translated_message != Some(chat_row.message.clone());

        // The chat message
        if is_translated {
            job.append(
                " ",
                0.0,
                TextFormat {
                    color: Color32::from_rgb(210, 210, 210),
                    ..Default::default()
                },
            );
            job.append(
                format!("({})", &chat_row.translated_message.as_ref().unwrap()).as_str(),
                0.0,
                TextFormat {
                    color: Color32::from_rgb(255, 255, 255),
                    background: Color32::from_rgb(20, 20, 20),
                    ..Default::default()
                },
            );
        }

        // Add the formatted text to the UI and make it clickable
        if ui
            .label(job)
            .on_hover_text(format!("Original message: {}", chat_row.message))
            .clicked()
        {
            app_win.set_selected_player(chat_row.steamid);
        }
    });
}
