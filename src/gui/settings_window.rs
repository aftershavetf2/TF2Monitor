use super::theme;
use crate::models::{
    AppWin, TempSettings,
    app_settings::{DEFAULT_LAUNCH_OPTIONS, DEFAULT_LOG_FILENAME},
    steamid::{MIN_STEAMID64, SteamID},
};
use eframe::egui::{self, TextEdit, Ui};

pub fn show_settings_window(app_win: &mut AppWin, ctx: &egui::Context) {
    if !app_win.settings_window_open {
        return;
    }

    let settings = &app_win.app_settings;
    let rcon_password_missing = settings.rcon_password.trim().is_empty();
    let temp = app_win.temp_settings.get_or_insert_with(|| TempSettings {
        self_steamid64: settings.self_steamid64.to_u64().to_string(),
        steam_api_key: settings.steam_api_key.clone(),
        log_filename: settings.log_filename.clone(),
        launch_options: settings.launch_options.clone(),
    });

    let mut window_open = true;
    let mut cancel = false;
    let mut save_steamid = None;
    theme::window(ctx, "Settings")
        .open(&mut window_open)
        .default_width(500.0)
        .default_height(520.0)
        .show(ctx, |ui| {
            let spacing = ui.spacing().item_spacing.y;
            let footer_height = ui.spacing().interact_size.y + 2.0 * spacing;
            let form_height = (ui.available_height() - footer_height).max(0.0);
            let steamid = theme::vertical_scroll_area_with_gutter(ui)
                .max_height(form_height)
                .show(ui, |ui| {
                    show_settings_content(ui, temp, rcon_password_missing)
                })
                .inner;

            ui.add_space(spacing);
            egui::containers::Sides::new().show(
                ui,
                |_| {},
                |ui| {
                    if ui
                        .add_enabled(steamid.is_some(), egui::Button::new("Save"))
                        .clicked()
                    {
                        save_steamid = steamid;
                    }
                    cancel = ui.add(theme::secondary_button("Cancel")).clicked();
                },
            );
        });

    if let Some(steamid) = save_steamid
        && let Some(temp) = app_win.temp_settings.take()
    {
        save_settings(app_win, temp, steamid);
        app_win.settings_window_open = false;
    } else if cancel || !window_open {
        app_win.settings_window_open = false;
        app_win.temp_settings = None;
    }
}

fn show_settings_content(
    ui: &mut Ui,
    temp: &mut TempSettings,
    rcon_password_missing: bool,
) -> Option<SteamID> {
    theme::section_heading(ui, "Steam");
    ui.label("Your Steam ID").on_hover_text(
        "Use the Steam account you play TF2 with. This identifies you on the scoreboard.",
    );
    ui.add(TextEdit::singleline(&mut temp.self_steamid64).desired_width(f32::INFINITY));
    let steamid = parse_steam_id(&temp.self_steamid64);
    if let Some(steamid) = steamid {
        ui.hyperlink_to("View Steam Profile", steamid.steam_community_url());
    } else {
        ui.colored_label(
            ui.visuals().error_fg_color,
            "Enter your SteamID64 or SteamID3 (for example, [U:1:12345]).",
        );
    }

    ui.add_space(ui.spacing().item_spacing.y);
    ui.label("Steam API Key")
        .on_hover_text("Optional. Enables profile information from Steam Community.");
    ui.add(TextEdit::singleline(&mut temp.steam_api_key).desired_width(f32::INFINITY));
    ui.hyperlink_to(
        "Get a Steam API Key",
        "https://steamcommunity.com/dev/apikey",
    );

    ui.add_space(ui.spacing().item_spacing.y);
    theme::section_heading(ui, "Team Fortress 2");
    if rcon_password_missing {
        ui.colored_label(
            ui.visuals().warn_fg_color,
            "RCON password is missing. Set rcon_password in settings.json, then restart TF2Monitor.",
        );
    }

    field_label_with_default(ui, "Log File", &mut temp.log_filename, DEFAULT_LOG_FILENAME);
    ui.add(TextEdit::singleline(&mut temp.log_filename).desired_width(f32::INFINITY));

    field_label_with_default(
        ui,
        "Launch Options",
        &mut temp.launch_options,
        DEFAULT_LAUNCH_OPTIONS,
    );
    ui.add(
        TextEdit::multiline(&mut temp.launch_options)
            .desired_width(f32::INFINITY)
            .desired_rows(3),
    );

    steamid
}

fn field_label_with_default(ui: &mut Ui, label: &str, value: &mut String, default: &str) {
    egui::containers::Sides::new().shrink_left().wrap().show(
        ui,
        |ui| ui.label(label),
        |ui| {
            if ui
                .add_enabled(value != default, theme::secondary_button("Set Default"))
                .clicked()
            {
                value.replace_range(.., default);
            }
        },
    );
}

fn parse_steam_id(input: &str) -> Option<SteamID> {
    let input = input.trim();
    if let Some(account) = input
        .strip_prefix("[U:1:")
        .and_then(|id| id.strip_suffix(']'))
    {
        let account = account.parse::<u32>().ok()?;
        return Some(SteamID::from_u64(MIN_STEAMID64 + u64::from(account)));
    }

    let id = input.parse::<u64>().ok()?;
    let range = MIN_STEAMID64..=MIN_STEAMID64 + u64::from(u32::MAX);
    range.contains(&id).then(|| SteamID::from_u64(id))
}

fn save_settings(app_win: &mut AppWin, temp: TempSettings, steamid: SteamID) {
    app_win.app_settings.self_steamid64 = steamid;
    app_win.app_settings.steam_api_key = temp.steam_api_key;
    app_win.app_settings.log_filename = temp.log_filename;
    app_win.app_settings.launch_options = temp.launch_options;
    app_win.self_steamid = steamid;
    app_win.updated_settings();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steam_id_field_accepts_both_formats_and_rejects_incomplete_input() {
        let expected = SteamID::from_u64(MIN_STEAMID64 + 12345);
        assert_eq!(
            parse_steam_id(&format!("  {}  ", expected.to_u64())),
            Some(expected)
        );
        assert_eq!(parse_steam_id("[U:1:12345]"), Some(expected));

        for invalid in [
            "",
            "0",
            "[U:1:",
            "[U:1:12345",
            "[U:1:abc]",
            "[U:1:4294967296]",
            "18446744073709551615",
        ] {
            assert_eq!(parse_steam_id(invalid), None, "{invalid}");
        }
    }
}
