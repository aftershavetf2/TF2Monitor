use crate::{models::AppWin, tf2::lobby::Player};
use eframe::egui::{Color32, Pos2, Rect, Stroke, Ui, Vec2};

pub fn add_friendship_indicators(app_win: &mut AppWin, ui: &mut Ui) {
    let stroke = Stroke::new(1.0f32, Color32::WHITE);

    for player in app_win.lobby.players.iter() {
        if let Some(steam_info) = &player.steam_info {
            if let Some(friends) = &steam_info.friends {
                if let Some(start_pos) = find_pos_for_player(app_win, player) {
                    for (steamid, end_pos) in app_win.friendship_positions.iter() {
                        if friends.contains(steamid) {
                            draw_curve(ui, start_pos, *end_pos, &stroke);
                        }
                    }
                }
            }
        }
    }
}

fn find_pos_for_player(app_win: &AppWin, player: &Player) -> Option<Pos2> {
    for (id, pos) in app_win.friendship_positions.iter() {
        if id == &player.steamid {
            return Some(*pos);
        }
    }
    None
}

fn draw_curve(ui: &mut Ui, start_pos: Pos2, end_pos: Pos2, stroke: &Stroke) {
    let r = 3.0f32;
    ui.painter().circle_filled(start_pos, r, stroke.color);
    ui.painter().circle_filled(end_pos, r, stroke.color);

    if start_pos.x != end_pos.x {
        // These are lines that are between players in different teams
        ui.painter().line_segment([start_pos, end_pos], *stroke);
        return;
    }

    // These are lines between players in the same team
    const NSEGS: usize = 6;
    const XES: [f32; NSEGS] = [0.0, 7.0, 10.0, 12.0, 10.0, 7.0];

    let y_delta = end_pos.y - start_pos.y;
    let y_delta_inc = y_delta / NSEGS as f32;

    let x_scale = 1.0 + y_delta.abs() / 250.0;

    let mut a = Pos2::new(0f32, 0f32);
    let mut b = Pos2::new(0f32, 0f32);
    for i in 0..NSEGS {
        a.y = start_pos.y + i as f32 * y_delta_inc;
        b.y = start_pos.y + (i + 1) as f32 * y_delta_inc;

        a.x = start_pos.x - x_scale * XES[i];
        b.x = start_pos.x - x_scale * XES[(i + 1) % NSEGS];

        ui.painter().line_segment([a, b], *stroke);
    }
}
