use super::{
    menu_item,
    theme::{self, MenuTooltip},
};
use crate::{models::AppWin, tf2bd::models::PlayerAttribute, utils::print_image_data};
use eframe::egui::{Ui, ViewportCommand};
use std::process::Command;

/*
Menu structure:

File
- Settings...
- Start TF2
- Quit

View
- [x] Show Friendsips
- [x] Show Crits

Actions
- [ ] Kick Cheaters
- [x] Kick Bots

Rules(todo)
- Make selected avatar as Bot

*/

pub fn add_top_menu(ui: &mut Ui, app_win: &mut AppWin) {
    theme::menu_bar().ui(ui, |ui| {
        theme::menu_header(ui, "File", |ui| {
            theme::menu_contents(ui);
            if menu_item::command(ui, "Start TF2").clicked() {
                start_tf2(app_win);
                ui.close();
            }

            ui.separator();

            if menu_item::command_with_keys(ui, "Settings...", &[super::shortcut::command_label(), ","]).clicked() {
                app_win.settings_window_open = true;
                ui.close();
            }

            ui.separator();

            if menu_item::command_with_keys(ui, "Quit", &[super::shortcut::command_label(), "Q"]).clicked() {
                ui.ctx().send_viewport_cmd(ViewportCommand::Close);
            }
        });

        theme::menu_header(ui, "View", |ui| {
            theme::menu_contents(ui);
            if menu_item::checkbox(ui, &mut app_win.app_settings.show_crits, "Show Crits")
                .changed()
            {
                app_win.updated_settings();
            }

            if menu_item::checkbox(ui,
                    &mut app_win.app_settings.show_friendship_indicators,
                    "Show Friendships",
                )
                .changed()
            {
                app_win.updated_settings();
            }

            if menu_item::checkbox(ui, &mut app_win.app_settings.show_health, "Show Health")
                .changed()
            {
                app_win.updated_settings();
            }

            if menu_item::checkbox(ui, &mut app_win.app_settings.show_ping, "Show Ping")
                .changed()
            {
                app_win.updated_settings();
            }

            if menu_item::checkbox(ui, &mut app_win.app_settings.show_weapons, "Show Weapons")
                .changed()
            {
                app_win.updated_settings();
            }
        });

        theme::menu_header(ui, "Actions", |ui| {
            theme::menu_contents(ui);
            if menu_item::checkbox(ui, &mut app_win.app_settings.translate_chat, "Translate Chat")
                .menu_tooltip("Translate chat messages to English using Google Translate")
                .changed()
            {
                app_win.updated_settings();
            }

            ui.separator();

            if menu_item::checkbox(ui, &mut app_win.app_settings.kick_cheaters, "Kick Cheaters")
                .changed()
            {
                app_win.updated_settings();
            }

            if menu_item::checkbox(ui, &mut app_win.app_settings.kick_bots, "Kick Bots")
                .changed()
            {
                app_win.updated_settings();
            }

            ui.separator();

            ui.label("Notify party about joining:");

            let player_attributes_to_show = vec![
                // PlayerAttribute::Cool,
                PlayerAttribute::Cheater,
                PlayerAttribute::Bot,
                PlayerAttribute::Suspicious,
                PlayerAttribute::Toxic,
                PlayerAttribute::Exploiter,
            ];

            for player_attribute in player_attributes_to_show {
                let mut enabled = app_win
                    .app_settings
                    .party_notifications_for
                    .contains(&player_attribute);
                if menu_item::checkbox(ui, &mut enabled, &format!("{:?}", player_attribute))
                    .changed()
                {
                    app_win
                        .app_settings
                        .party_notifications_for
                        .retain(|&x| x != player_attribute);

                    if enabled {
                        app_win
                            .app_settings
                            .party_notifications_for
                            .push(player_attribute);
                    }

                    app_win.updated_settings();
                }
            }
        });

        theme::menu_header(ui, "Tools", |ui| {
            theme::menu_contents(ui);
            if menu_item::checkbox(ui, &mut app_win.spectating, "Spectate Players")
                .menu_tooltip("Allows you to spectate your team members.\nIt sends 'kill; menuopen' and 'menuclosed' to rcon")
                .changed()
            {
                let cmd = if app_win.spectating {
                    "kill; menuopen"
                } else {
                    "menuclosed"
                };

                app_win.bus.lock().unwrap().send_rcon_cmd(cmd);
            }

            if menu_item::command(ui, "Restart Sound")
            .menu_tooltip("Restarts the sound engine to get rid of stuck sound loops.\nIt sends 'snd_restart' to rcon").clicked() {
                app_win.bus.lock().unwrap().send_rcon_cmd("snd_restart");
            }

            if menu_item::command(ui, "Reload HUD")
                .menu_tooltip("Reloads the HUD, useful when it's broken.\nIt sends 'hud_reloadscheme' to rcon")
            .clicked() {
                app_win
                    .bus
                    .lock()
                    .unwrap()
                    .send_rcon_cmd("hud_reloadscheme");
            }

            if menu_item::command(ui, "Fix Invisible Players")
            .menu_tooltip("Fixes invisible players by starting a demo recording and stops it").clicked() {
                app_win
                    .bus
                    .lock()
                    .unwrap()
                    .send_rcon_cmd("record fix; stop");
            }

            ui.separator();

            if menu_item::command(ui, "DB Statistics").clicked() {
                app_win.db_statistics_window_open = true;
                ui.close();
            }

            if menu_item::command(ui, "Player Database").clicked() {
                app_win.player_database_window_open = true;
                ui.close();
            }

            ui.separator();

            if menu_item::command(ui, "Dump Debug Info")
                .menu_tooltip("Dumps debug info to the console window, right now it's just the player avatars")
                .clicked() {

                    app_win.get_latest_lobby();

                    let lobby = &app_win.lobby;
                    let players = &lobby.players;

                    log::info!("Dumping debug info for {} players", players.len());

                    for player in players {
                        if let Some(steam_info) = &player.steam_info {
                            let other_url = &steam_info.avatar_thumb;
                            // let other_url = "https://old.reddit.com/r/funnyvideos/";

                            print_image_data(other_url);
                        } else {
                            log::info!(
                                "Player {} has no steam info, skipping avatar dump",
                                player.name
                            );
                        }
                    }
                    ui.close();
            }
        });
        super::title_bar::contents(ui, "TF2 Monitor");
    });
}

fn start_tf2(app_win: &AppWin) {
    let exe_path = &app_win.app_settings.exe_filename;
    let launch_options = &app_win.app_settings.launch_options;

    log::info!("Starting TF2 with exe: {}", exe_path);
    log::info!("Launch options: {}", launch_options);

    // Build the command with -steam -game tf and then the launch options
    let mut cmd = Command::new(exe_path);
    cmd.arg("-steam").arg("-game").arg("tf");

    // Parse and add launch options
    if !launch_options.is_empty() {
        // Split launch options by spaces, but be careful with quoted strings
        let options: Vec<&str> = launch_options.split_whitespace().collect();
        for option in options {
            cmd.arg(option);
        }
    }

    // Spawn the process
    match cmd.spawn() {
        Ok(mut child) => {
            log::info!("TF2 started successfully with PID: {:?}", child.id());
            // Detach the child process so it continues running independently
            let _ = child;
        }
        Err(e) => {
            log::error!("Failed to start TF2: {}", e);
        }
    }
}

/// App shortcuts are ignored while a text field or modal owns keyboard input.
pub fn handle_shortcuts(ctx: &eframe::egui::Context, app: &mut AppWin) {
    use eframe::egui::Key;
    if super::shortcut::consume_command(ctx, Key::Comma) {
        app.settings_window_open = true;
    }
    if super::shortcut::consume_command(ctx, Key::Q) {
        ctx.send_viewport_cmd(ViewportCommand::Close);
    }
}
