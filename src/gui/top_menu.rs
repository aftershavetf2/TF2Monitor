use super::{
    colors::TF2BD_BADGE_COLORS,
    menu_item,
    theme::{self, MenuTooltip},
};
use crate::{
    models::AppWin, tf2::lobby::player_attribute_description, tf2bd::models::PlayerAttribute,
};
use eframe::egui::{Ui, ViewportCommand};

const SHOW_TRANSLATE_CHAT: bool = false;

pub fn add_top_menu(ui: &mut Ui, app_win: &mut AppWin) {
    theme::menu_bar().ui(ui, |ui| {
        theme::menu_header(ui, "File", |ui| add_file_menu(ui, app_win));
        theme::menu_header(ui, "View", |ui| add_view_menu(ui, app_win));
        theme::menu_header(ui, "Actions", |ui| add_actions_menu(ui, app_win));
        theme::menu_header(ui, "Tools", |ui| add_tools_menu(ui, app_win));
        super::title_bar::contents(ui, "TF2 Monitor");
    });
}

fn add_file_menu(ui: &mut Ui, app_win: &mut AppWin) {
    theme::menu_contents(ui);
    if menu_item::command_with_keys(ui, "Settings...", &[super::shortcut::command_label(), ","])
        .clicked()
    {
        app_win.settings_window_open = true;
        ui.close();
    }

    ui.separator();
    if menu_item::command(ui, "Import Player Markings...").clicked() {
        app_win.marking_transfer = Some(super::marking_transfer::Transfer::new(true));
    }
    if menu_item::command(ui, "Export Player Markings...").clicked() {
        app_win.marking_transfer = Some(super::marking_transfer::Transfer::new(false));
    }

    ui.separator();

    if menu_item::command_with_keys(ui, "Quit", &[super::shortcut::command_label(), "Q"]).clicked()
    {
        ui.ctx().send_viewport_cmd(ViewportCommand::Close);
    }
}

fn add_view_menu(ui: &mut Ui, app_win: &mut AppWin) {
    theme::menu_contents(ui);
    super::tabs::add_menu_section(&mut app_win.active_tab, ui);
    ui.separator();
    if menu_item::checkbox(ui, &mut app_win.app_settings.show_crits, "Show Crits").changed() {
        app_win.updated_settings();
    }

    if menu_item::checkbox(
        ui,
        &mut app_win.app_settings.show_friendship_indicators,
        "Show Friendships",
    )
    .changed()
    {
        app_win.updated_settings();
    }

    if menu_item::checkbox(ui, &mut app_win.app_settings.show_health, "Show Health").changed() {
        app_win.updated_settings();
    }

    if menu_item::checkbox(ui, &mut app_win.app_settings.show_ping, "Show Ping").changed() {
        app_win.updated_settings();
    }

    if menu_item::checkbox(ui, &mut app_win.app_settings.show_weapons, "Show Weapons").changed() {
        app_win.updated_settings();
    }
}

fn add_actions_menu(ui: &mut Ui, app_win: &mut AppWin) {
    theme::menu_contents(ui);
    if SHOW_TRANSLATE_CHAT {
        ui.add_enabled_ui(false, |ui| {
            menu_item::checkbox(
                ui,
                &mut app_win.app_settings.translate_chat,
                "Translate Chat",
            )
            .menu_tooltip("Translate chat messages to English using Google Translate");
        });
        ui.separator();
    }

    if menu_item::flag_checkbox(
        ui,
        &mut app_win.app_settings.kick_cheaters,
        "Kick Cheaters",
        PlayerAttribute::Cheater,
    )
    .changed()
    {
        app_win.updated_settings();
    }

    if menu_item::flag_checkbox(
        ui,
        &mut app_win.app_settings.kick_bots,
        "Kick Bots",
        PlayerAttribute::Bot,
    )
    .changed()
    {
        app_win.updated_settings();
    }

    ui.separator();

    add_party_notifications(ui, app_win);
}

fn add_party_notifications(ui: &mut Ui, app_win: &mut AppWin) {
    ui.label("Notify party about joining:");

    if menu_item::colored_checkbox(
        ui,
        &mut app_win.app_settings.notify_tf2bd,
        "TF2BD",
        TF2BD_BADGE_COLORS,
    )
    .menu_tooltip("Notify your party when a joining player matches the active TF2BD list filters.")
    .changed()
    {
        app_win.updated_settings();
    }

    let player_attributes_to_show = [
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
        let (_, label) = player_attribute_description(player_attribute);
        if menu_item::flag_checkbox(ui, &mut enabled, label, player_attribute).changed() {
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
}

fn add_tools_menu(ui: &mut Ui, app_win: &mut AppWin) {
    theme::menu_contents(ui);
    add_game_commands(ui, app_win);

    ui.separator();

    menu_item::submenu(ui, "SourceBan Data", |ui| {
        source_ban_data_menu(ui, app_win);
    });

    if menu_item::command(ui, "TF2BD Player Lists").clicked() {
        app_win.tf2bd_lists_window.open = true;
        ui.close();
    }

    ui.separator();

    if menu_item::command(ui, "Player Database").clicked() {
        app_win.player_database_window_open = true;
        ui.close();
    }

    if menu_item::command(ui, "DB Statistics").clicked() {
        app_win.db_statistics_window_open = true;
        ui.close();
    }
}

fn add_game_commands(ui: &mut Ui, app_win: &mut AppWin) {
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
        .menu_tooltip(
            "Reloads the HUD, useful when it's broken.\nIt sends 'hud_reloadscheme' to rcon",
        )
        .clicked()
    {
        app_win
            .bus
            .lock()
            .unwrap()
            .send_rcon_cmd("hud_reloadscheme");
    }

    if menu_item::command(ui, "Fix Invisible Players")
        .menu_tooltip("Fixes invisible players by starting a demo recording and stops it")
        .clicked()
    {
        app_win
            .bus
            .lock()
            .unwrap()
            .send_rcon_cmd("record fix; stop");
    }
}

fn source_ban_data_menu(ui: &mut Ui, app_win: &AppWin) {
    ui.set_width(340.0);
    let ban_data = app_win.bus.lock().unwrap().ban_data.clone();
    let status = ban_data.status();
    ui.add_enabled_ui(status.available && !status.busy, |ui| {
        if menu_item::command(ui, "Check for Updates").clicked() {
            ban_data.request_check();
        }
    });
    ui.separator();
    menu_item::information(ui, |ui| {
        let time = status
            .state
            .last_attempt
            .and_then(|value| chrono::DateTime::from_timestamp(value, 0))
            .map(|value| {
                let local = value.with_timezone(&chrono::Local);
                if local.date_naive() == chrono::Local::now().date_naive() {
                    local.format("Today at %H:%M").to_string()
                } else {
                    local.format("%Y-%m-%d %H:%M").to_string()
                }
            })
            .unwrap_or_else(|| "Never".into());
        ui.label(format!("Last checked at: {time}"));
    });
}

/// App shortcuts are ignored while a text field or modal owns keyboard input.
pub fn handle_shortcuts(ctx: &eframe::egui::Context, app: &mut AppWin) {
    use eframe::egui::Key;
    super::tabs::handle_shortcuts(&mut app.active_tab, ctx);
    if super::shortcut::consume_command(ctx, Key::Comma) {
        app.settings_window_open = true;
    }
    if super::shortcut::consume_command(ctx, Key::Q) {
        ctx.send_viewport_cmd(ViewportCommand::Close);
    }
}
