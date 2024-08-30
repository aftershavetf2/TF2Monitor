use crate::{models::AppWin, tf2bd::models::PlayerAttribute};
use eframe::egui::{menu, TextBuffer, Ui, ViewportCommand};

/*
Menu structure:

File
- Settings...
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
    menu::bar(ui, |ui| {
        ui.menu_button("File", |ui| {
            if ui.button("Quit").clicked() {
                ui.ctx().send_viewport_cmd(ViewportCommand::Close);
            }
        });

        ui.menu_button("View", |ui| {
            if ui
                .checkbox(&mut app_win.app_settings.show_crits, "Show crits")
                .changed()
            {
                app_win.updated_settings();
            }

            if ui
                .checkbox(
                    &mut app_win.app_settings.show_friendship_indicators,
                    "Show friendships",
                )
                .changed()
            {
                app_win.updated_settings();
            }
        });

        ui.menu_button("Actions", |ui| {
            if ui
                .checkbox(&mut app_win.app_settings.kick_cheaters, "Kick cheaters")
                .changed()
            {
                app_win.updated_settings();
            }

            if ui
                .checkbox(&mut app_win.app_settings.kick_bots, "Kick bots")
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
                if ui
                    .checkbox(&mut enabled, format!(" {:?}", player_attribute))
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

        ui.menu_button("Tools", |ui| {
            if ui
                .checkbox(&mut app_win.spectating, "Spectate players")
                .changed()
            {
                let cmd = if app_win.spectating {
                    "kill; menuopen"
                } else {
                    "menuclosed"
                };

                app_win.bus.lock().unwrap().send_rcon_cmd(cmd);
            }

            if ui.button("Restart sound").clicked() {
                app_win.bus.lock().unwrap().send_rcon_cmd("snd_restart");
            }
        });
    });
}
