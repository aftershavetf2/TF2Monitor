use crate::models::AppWin;
use eframe::egui::{Color32, Pos2, Stroke, Ui};

pub fn add_friendship_indicators(app_win: &mut AppWin, ui: &mut Ui) {
    let indicator_color = Color32::WHITE;
    let stroke = Stroke::new(0.6f32, indicator_color);

    let me = app_win.self_steamid;
    let my_friends = app_win.lobby.friendships.get_friends(me);

    // Loop through all players and draw lines between their friends
    for player in app_win.lobby.players.iter() {
        // Lines from me are not drawn
        if player.steamid == me {
            continue;
        }

        let friends = app_win.lobby.friendships.get_friends(player.steamid);

        if let Some(start_pos) = app_win.friendship_positions.get(&player.steamid) {
            for steamid in friends {
                // Friendship is bidirectional, so only draw the line one way,
                if steamid.to_u64() > player.steamid.to_u64() {
                    continue;
                }

                // Lines to me are not drawn
                if *steamid == me {
                    continue;
                }

                // Lines between two of my friends are not drawn
                if my_friends.contains(steamid) && my_friends.contains(&player.steamid) {
                    continue;
                }

                if let Some(end_pos) = app_win.friendship_positions.get(steamid) {
                    // Draw a line between the two players
                    // The left/right direction of the line
                    // depends on the two steamids
                    let dir = 1 == (player.steamid.to_u64() ^ steamid.to_u64()) & 1;
                    draw_curve(ui, *start_pos, *end_pos, &stroke, dir);
                }
            }
        }
    }
}

fn draw_curve(ui: &mut Ui, start_pos: Pos2, end_pos: Pos2, stroke: &Stroke, dir: bool) {
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
    const XES: [f32; NSEGS] = [0.0, 6.0, 9.0, 9.5, 9.0, 6.0];

    let y_delta = end_pos.y - start_pos.y;
    let y_delta_inc = y_delta / NSEGS as f32;

    let mut x_scale = 1.0 + y_delta.abs() / 150.0;

    if dir {
        x_scale = -x_scale;
    }

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
