use crate::models::{AppWin, TempSettings};
use eframe::egui::{self, Grid, Ui};

/// Shows the settings window as an egui Window (internal frame)
pub fn show_settings_window(app_win: &mut AppWin, ctx: &egui::Context) {
    if !app_win.settings_window_open {
        return;
    }

    // Initialize temp settings if not already done
    if app_win.temp_settings.is_none() {
        app_win.temp_settings = Some(TempSettings {
            self_steamid64: app_win.app_settings.self_steamid64.to_u64().to_string(),
            steam_api_key: app_win.app_settings.steam_api_key.clone(),
            rcon_password: app_win.app_settings.rcon_password.clone(),
            rcon_ip: app_win.app_settings.rcon_ip.clone(),
            rcon_port: app_win.app_settings.rcon_port.to_string(),
            log_filename: app_win.app_settings.log_filename.clone(),
            exe_filename: app_win.app_settings.exe_filename.clone(),
        });
    }

    let mut window_open = app_win.settings_window_open;

    let mut should_close = false;
    let mut should_save = false;

    egui::Window::new("Settings")
        .open(&mut window_open)
        .resizable(true)
        .default_width(500.0)
        .show(ctx, |ui| {
            // Show warning message if configuration is incomplete
            if !app_win.app_settings.config_is_ok {
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    "⚠ Configuration is incomplete. Please fill in the required fields below.",
                );
                ui.add_space(10.0);
            }

            if let Some(ref mut temp) = app_win.temp_settings {
                show_settings_content(ui, temp);

                ui.separator();

                // Buttons at the bottom - aligned to the right
                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                    if ui.button("Cancel").clicked() {
                        should_close = true;
                    }

                    if ui.button("Save").clicked() {
                        should_save = true;
                    }
                });
            }
        });

    // Handle button actions outside the window closure
    if should_save {
        if let Some(temp) = app_win.temp_settings.clone() {
            save_settings(app_win, &temp);
        }
        app_win.settings_window_open = false;
        app_win.temp_settings = None;
    }

    if should_close {
        app_win.settings_window_open = false;
        app_win.temp_settings = None;
    }

    // Update the window open state (handles X button click)
    if !window_open {
        app_win.settings_window_open = false;
        app_win.temp_settings = None;
    }
}

fn show_settings_content(ui: &mut Ui, temp: &mut TempSettings) {
    use crate::models::steamid::SteamID;

    let info_label_color = egui::Color32::GRAY;

    Grid::new("settings_grid")
        .num_columns(2)
        .spacing([10.0, 16.0])
        .min_col_width(200.0)
        .show(ui, |ui| {
            // self_steamid64
            ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                ui.label("Self SteamID64:");
            });
            ui.vertical(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut temp.self_steamid64)
                        .desired_width(f32::INFINITY),
                );

                ui.add_space(5.0);

                // Show Steam Community link for verification
                let steamid = if let Some(id) = SteamID::from_u64_string(&temp.self_steamid64) {
                    id
                } else if temp.self_steamid64.starts_with("[U:1:") {
                    SteamID::from_steam_id32(&temp.self_steamid64)
                } else {
                    SteamID::from_u64(0)
                };

                if steamid.is_valid() {
                    let url = steamid.steam_community_url();
                    ui.hyperlink_to("View Profile on Steam Community", &url);
                } else {
                    ui.label("(Invalid SteamID)");
                }

                ui.add_space(10.0);
                ui.colored_label(info_label_color, "This is your SteamID64. You can find it in your Steam profile URL. It identifies you on the scoreboard.");

            });
            ui.end_row();

            // steam_api_key
            ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                ui.label("Steam API Key:");
            });
            ui.vertical(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut temp.steam_api_key)
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(5.0);
                ui.hyperlink_to(
                    "Go to Steam API Key page",
                    "https://steamcommunity.com/dev/apikey",
                );
                ui.add_space(10.0);
                ui.colored_label(info_label_color, "Get your personal Steam API key from the Steam API Key page. It is required to fetch profile information from Steam Community.");
            });
            ui.end_row();

            // rcon_password
            // ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
            //     ui.label("RCON Password:");
            // });
            // ui.add(
            //     egui::TextEdit::singleline(&mut temp.rcon_password).desired_width(f32::INFINITY),
            // );
            // ui.end_row();

            // rcon_ip
            // ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
            //     ui.label("RCON IP:");
            // });
            // ui.vertical(|ui| {
            //     ui.add(egui::TextEdit::singleline(&mut temp.rcon_ip).desired_width(f32::INFINITY));
            //     ui.add_space(10.0);
            //     ui.label("This is the IP address of the server you want to connect to. It is used to send commands to the server.");
            // });
            // ui.add(egui::TextEdit::singleline(&mut temp.rcon_ip).desired_width(f32::INFINITY));
            // ui.end_row();
            
            // rcon_port
            // ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
            //     ui.label("RCON Port:");
            // });
            // ui.add(egui::TextEdit::singleline(&mut temp.rcon_port).desired_width(f32::INFINITY));
            // ui.end_row();

            // log_filename
            ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                ui.label("TF2Log Filename:");
            });
            ui.vertical(|ui| {
                ui.add(egui::TextEdit::singleline(&mut temp.log_filename).desired_width(f32::INFINITY));
                ui.add_space(10.0);
                ui.colored_label(info_label_color, "The TF2 log file that TF2Monitor reads while you play.");
                ui.add_space(5.0);
                ui.colored_label(info_label_color, "Default is C:\\Program Files (x86)\\Steam\\steamapps\\common\\Team Fortress 2\\tf\\console.log");
            });
            ui.end_row();

            // exe_filename
            // ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
            //     ui.label("TF2 Exe Filename:");
            // });
            // ui.vertical(|ui| {
            //     ui.add(egui::TextEdit::singleline(&mut temp.exe_filename).desired_width(f32::INFINITY));
            //     ui.add_space(10.0);
            //     ui.colored_label(info_label_color, "This is the filename of the TF2 executable that TF2Monitor will use to start TF2. Default is C:\\Program Files (x86)\\Steam\\steamapps\\common\\Team Fortress 2\\tf_win64.exe.");
            // });
            // ui.end_row();
        });
}

fn save_settings(app_win: &mut AppWin, temp: &TempSettings) {
    use crate::models::steamid::SteamID;

    // Parse and validate the settings
    let steamid = if let Some(id) = SteamID::from_u64_string(&temp.self_steamid64) {
        id
    } else {
        // Try parsing as SteamID32 format [U:1:xxxxx]
        if temp.self_steamid64.starts_with("[U:1:") {
            SteamID::from_steam_id32(&temp.self_steamid64)
        } else {
            log::warn!("Invalid SteamID format, keeping old value");
            app_win.app_settings.self_steamid64
        }
    };

    let rcon_port: u16 = temp.rcon_port.parse().unwrap_or_else(|_| {
        log::warn!("Invalid rcon_port, keeping old value");
        app_win.app_settings.rcon_port
    });

    // Update app_settings
    app_win.app_settings.self_steamid64 = steamid;
    app_win.app_settings.steam_api_key = temp.steam_api_key.clone();
    app_win.app_settings.rcon_password = temp.rcon_password.clone();
    app_win.app_settings.rcon_ip = temp.rcon_ip.clone();
    app_win.app_settings.rcon_port = rcon_port;
    app_win.app_settings.log_filename = temp.log_filename.clone();
    app_win.app_settings.exe_filename = temp.exe_filename.clone();

    // Also update self_steamid in AppWin
    app_win.self_steamid = steamid;

    // Save and broadcast
    app_win.updated_settings();

    log::info!("Settings updated and saved");
}
