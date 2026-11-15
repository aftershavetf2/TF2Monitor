use super::{
    chat::add_chat, friendship_indicators::add_friendship_indicators, kill_feed::add_kill_feed,
    scoreboard::add_scoreboard,
};
use crate::models::AppWin;
use eframe::egui::Ui;

pub fn show(app_win: &mut AppWin, ui: &mut Ui) {
    add_scoreboard(app_win, ui);

    ui.separator();

    ui.columns(2, |columns| {
        add_chat(&mut columns[0], app_win);
        add_kill_feed(&mut columns[1], app_win);
    });

    if app_win.app_settings.show_friendship_indicators {
        add_friendship_indicators(app_win, ui);
    }
}
