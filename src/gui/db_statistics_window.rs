use crate::models::AppWin;
use eframe::egui::{self, Ui};

/// Shows the DB statistics window as an egui Window (internal frame)
pub fn show_db_statistics_window(app_win: &mut AppWin, ctx: &egui::Context) {
    if !app_win.db_statistics_window_open {
        return;
    }

    let mut window_open = app_win.db_statistics_window_open;

    super::theme::window(ctx, "Database Statistics")
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
    show_stat(ui, "Cached Players", get_total_accounts_count(&mut conn));

    ui.separator();
    ui.add_space(5.0);
    ui.heading("Player Flags");
    ui.add_space(5.0);

    use crate::tf2bd::models::PlayerAttribute;
    let markings = app_win.markings.snapshot();
    for (label, flag) in [
        ("Cheaters", PlayerAttribute::Cheater),
        ("Bots", PlayerAttribute::Bot),
        ("Suspicious", PlayerAttribute::Suspicious),
        ("Toxic", PlayerAttribute::Toxic),
        ("Exploiter", PlayerAttribute::Exploiter),
        ("Cool", PlayerAttribute::Cool),
    ] {
        let count = markings
            .values()
            .filter(|marking| marking.flags.contains(&flag))
            .count();
        show_stat(ui, label, Ok(count as i64));
    }

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
}
