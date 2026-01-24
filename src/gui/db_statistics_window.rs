use crate::models::AppWin;
use eframe::egui::{self, Ui};
use std::time::{SystemTime, UNIX_EPOCH};

/// Shows the DB statistics window as an egui Window (internal frame)
pub fn show_db_statistics_window(app_win: &mut AppWin, ctx: &egui::Context) {
    if !app_win.db_statistics_window_open {
        return;
    }

    let mut window_open = app_win.db_statistics_window_open;

    egui::Window::new("Database Statistics")
        .open(&mut window_open)
        .resizable(true)
        .default_width(400.0)
        .default_height(500.0)
        .show(ctx, |ui| {
            show_statistics_content(ui, app_win);
        });

    // Update the window open state (handles X button click)
    if !window_open {
        app_win.db_statistics_window_open = false;
    }
}

fn show_statistics_content(ui: &mut Ui, app_win: &AppWin) {
    use crate::db::queries::*;

    // Get current time for ban queries
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // Try to get a database connection
    let mut conn = match app_win.db.get() {
        Ok(conn) => conn,
        Err(e) => {
            ui.colored_label(
                ui.visuals().error_fg_color,
                format!("Database error: {}", e),
            );
            return;
        }
    };

    // ui.heading("Database Statistics");
    // ui.separator();
    // ui.add_space(10.0);

    // Helper function to display a statistic
    fn show_stat(ui: &mut Ui, label: &str, value: Result<i64, diesel::result::Error>) {
        match value {
            Ok(count) => {
                ui.horizontal(|ui| {
                    ui.label(label);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(format!("{}", count));
                    });
                });
            }
            Err(e) => {
                ui.colored_label(
                    ui.visuals().error_fg_color,
                    format!("{}: Error - {}", label, e),
                );
            }
        }
    }

    // Total accounts
    show_stat(ui, "Total Players", get_total_accounts_count(&mut conn));

    ui.separator();
    ui.add_space(5.0);
    ui.heading("Player Flags");
    ui.add_space(5.0);

    // Player flags - using the string representation from PlayerAttribute Debug
    show_stat(ui, "Cheaters", get_player_flag_count(&mut conn, "Cheater"));
    show_stat(ui, "Bots", get_player_flag_count(&mut conn, "Bot"));
    show_stat(
        ui,
        "Suspicious",
        get_player_flag_count(&mut conn, "Suspicious"),
    );
    show_stat(ui, "Toxic", get_player_flag_count(&mut conn, "Toxic"));
    show_stat(
        ui,
        "Exploiter",
        get_player_flag_count(&mut conn, "Exploiter"),
    );
    show_stat(ui, "Cool", get_player_flag_count(&mut conn, "Cool"));

    ui.separator();
    ui.add_space(5.0);
    ui.heading("Steam Bans");
    ui.add_space(5.0);

    show_stat(ui, "VAC Banned", get_vac_banned_count(&mut conn));
    show_stat(
        ui,
        "Community Banned",
        get_community_banned_count(&mut conn),
    );

    // ui.separator();
    // ui.add_space(5.0);
    // ui.heading("Other Statistics");
    // ui.add_space(5.0);

    // show_stat(ui, "Active Friendships", get_active_friendships_count(&mut conn));
    // show_stat(ui, "Active Comments", get_active_comments_count(&mut conn));
    // show_stat(ui, "Active Bans", get_active_bans_count(&mut conn, current_time));
}
