//! Spring layout of the friendships currently present on the server.
use super::{colors, markings, theme};
use crate::{
    models::{AppWin, steamid::SteamID},
    tf2::lobby::{Lobby, Player, Team},
};
use eframe::egui::{
    self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2, vec2,
};

const CARD_SIZE: Vec2 = vec2(280.0, 54.0);
// Bound the extra work in exceptionally dense friendship graphs. The cursor
// rotates through candidates so every connection is still considered over time.
const AVOIDANCE_BUDGET: usize = 8192;

struct Node {
    steamid: SteamID,
    position: Pos2,
    velocity: Vec2,
}

pub struct OverviewState {
    include_flagged_players: bool,
    card_size: Vec2,
    nodes: Vec<Node>,
    edges: Vec<(usize, usize)>,
    view: Rect,
    lobby_id: String,
    avoidance_cursor: usize,
}

impl Default for OverviewState {
    fn default() -> Self {
        Self {
            include_flagged_players: true,
            card_size: CARD_SIZE,
            nodes: Vec::new(),
            edges: Vec::new(),
            view: Rect::NOTHING,
            lobby_id: String::new(),
            avoidance_cursor: 0,
        }
    }
}

/// Sort by identity so incoming lobby updates do not shuffle the simulation.
fn graph(lobby: &Lobby, include_flagged_players: bool) -> (Vec<&Player>, Vec<(usize, usize)>) {
    let mut players: Vec<_> = lobby.players.iter().collect();
    players.sort_by_key(|player| player.steamid.to_u64());
    players.retain(|player| {
        players_have_friend(lobby, player)
            || (include_flagged_players && is_flagged_or_banned(player))
    });
    let mut edges = Vec::new();
    for (a, player) in players.iter().enumerate() {
        for (b, other) in players.iter().enumerate().skip(a + 1) {
            if lobby.friendships.are_friends(player.steamid, other.steamid) {
                edges.push((a, b));
            }
        }
    }
    (players, edges)
}

fn is_flagged_or_banned(player: &Player) -> bool {
    player
        .steam_bans
        .as_ref()
        .is_some_and(|b| b.vac_banned || b.number_of_game_bans > 0 || b.community_banned)
        || player
            .reputation
            .as_ref()
            .is_some_and(|r| !r.source_bans.is_empty())
        || player.player_info.as_ref().is_some_and(|p| {
            p.attributes
                .iter()
                .any(|flag| *flag != crate::tf2bd::models::PlayerAttribute::Cool)
        })
}

fn players_have_friend(lobby: &Lobby, player: &Player) -> bool {
    lobby.players.iter().any(|other| {
        other.steamid != player.steamid
            && lobby.friendships.are_friends(player.steamid, other.steamid)
    })
}

impl OverviewState {
    fn sync(&mut self, players: &[&Player], edges: Vec<(usize, usize)>, lobby_id: &str) {
        if self.lobby_id != lobby_id {
            *self = Self {
                lobby_id: lobby_id.to_owned(),
                include_flagged_players: self.include_flagged_players,
                card_size: self.card_size,
                ..Default::default()
            };
        }
        let changed = self.nodes.len() != players.len()
            || self
                .nodes
                .iter()
                .zip(players)
                .any(|(node, player)| node.steamid != player.steamid)
            || self.edges != edges;
        if !changed {
            return;
        }

        let radius = (players.len() as f32 * 40.0).max(180.0);
        self.nodes = players
            .iter()
            .enumerate()
            .map(|(index, player)| {
                if let Some(node) = self
                    .nodes
                    .iter()
                    .find(|node| node.steamid == player.steamid)
                {
                    return Node {
                        steamid: node.steamid,
                        position: node.position,
                        velocity: Vec2::ZERO,
                    };
                }
                let angle = index as f32 * std::f32::consts::TAU / players.len().max(1) as f32;
                Node {
                    steamid: player.steamid,
                    position: Pos2::ZERO + vec2(angle.cos(), angle.sin()) * radius,
                    velocity: Vec2::ZERO,
                }
            })
            .collect();
        self.edges = edges;
        self.avoidance_cursor = 0;
        // Settle the initial arrangement before fitting the camera.
        for _ in 0..180 {
            self.step(1.0 / 60.0, None);
        }
        self.fit();
    }

    fn fit(&mut self) {
        self.view = self
            .nodes
            .iter()
            .fold(Rect::NOTHING, |bounds, node| {
                bounds.union(Rect::from_center_size(node.position, self.card_size))
            })
            .expand(60.0);
    }

    fn step(&mut self, dt: f32, dragged: Option<usize>) -> bool {
        let mut forces = vec![Vec2::ZERO; self.nodes.len()];
        for a in 0..self.nodes.len() {
            for b in a + 1..self.nodes.len() {
                let delta = self.nodes[b].position - self.nodes[a].position;
                let distance = delta.length().max(1.0);
                let direction = if delta.length_sq() < 0.01 {
                    vec2(1.0, 0.5).normalized()
                } else {
                    delta / distance
                };
                let mut force = direction * (1_500_000.0 / distance.powi(2)).min(1800.0);
                // Keep the rectangular player cards apart, even in dense groups.
                let overlap = self.card_size + Vec2::splat(22.0) - delta.abs();
                if overlap.x > 0.0 && overlap.y > 0.0 {
                    force += if overlap.x < overlap.y {
                        vec2(if delta.x >= 0.0 { 1.0 } else { -1.0 }, 0.0) * overlap.x * 80.0
                    } else {
                        vec2(0.0, if delta.y >= 0.0 { 1.0 } else { -1.0 }) * overlap.y * 80.0
                    };
                }
                forces[a] -= force;
                forces[b] += force;
            }
        }
        for &(a, b) in &self.edges {
            let delta = self.nodes[b].position - self.nodes[a].position;
            let force = delta.normalized() * (delta.length() - (self.card_size.x + 51.0)) * 3.0;
            forces[a] += force;
            forces[b] -= force;
        }
        let mut avoidance = vec![Vec2::ZERO; self.nodes.len()];
        self.add_avoidance_forces(&mut avoidance);
        for (force, penalty) in forces.iter_mut().zip(avoidance) {
            // Keep collision separation stronger than accumulated crossing
            // penalties in dense/non-planar groups where crossings are inevitable.
            *force += penalty * (600.0 / penalty.length().max(600.0));
        }
        self.avoidance_cursor = self.avoidance_cursor.wrapping_add(AVOIDANCE_BUDGET);
        let mut moving = false;
        for (index, node) in self.nodes.iter_mut().enumerate() {
            if dragged == Some(index) {
                node.velocity = Vec2::ZERO;
                continue;
            }
            // A weak central pull keeps disconnected friend groups in one view.
            forces[index] -= node.position.to_vec2() * 0.12;
            node.velocity = (node.velocity + forces[index] * dt) * (-6.0 * dt).exp();
            let speed = node.velocity.length();
            if speed > 350.0 {
                node.velocity *= 350.0 / speed;
            }
            if node.velocity.length() > 0.3 {
                node.position += node.velocity * dt;
                moving = true;
            }
        }
        moving
    }

    fn add_avoidance_forces(&self, forces: &mut [Vec2]) {
        let edge_count = self.edges.len();
        let node_count = self.nodes.len();
        let card_checks = edge_count * node_count;
        for offset in 0..card_checks.min(AVOIDANCE_BUDGET) {
            let candidate = (self.avoidance_cursor.wrapping_add(offset)) % card_checks;
            let (a, b) = self.edges[candidate / node_count];
            let obstacle = candidate % node_count;
            if obstacle == a || obstacle == b {
                continue;
            }
            let start = self.nodes[a].position;
            let delta = self.nodes[b].position - start;
            if delta.length_sq() < 1.0 {
                continue;
            }
            let center = self.nodes[obstacle].position;
            let bounds = Rect::from_center_size(center, self.card_size).expand(14.0);
            let Some((entry, exit)) = segment_rect_interval(start, start + delta, bounds) else {
                continue;
            };
            let t = ((center - start).dot(delta) / delta.length_sq()).clamp(entry, exit);
            let closest = start + delta * t;
            let normal = vec2(-delta.y, delta.x).normalized();
            let distance = (center - closest).dot(normal);
            // Projection of the rectangular card onto the line's normal.
            let clearance = normal.abs().dot(self.card_size * 0.5) + 16.0;
            let side = if distance < 0.0 { -1.0 } else { 1.0 };
            let force = normal * side * ((clearance - distance.abs()) * 16.0).clamp(0.0, 1800.0);
            forces[obstacle] += force;
            forces[a] -= force * (1.0 - t);
            forces[b] -= force * t;
        }

        let pair_checks = edge_count * edge_count;
        for offset in 0..pair_checks.min(AVOIDANCE_BUDGET) {
            let candidate = (self.avoidance_cursor.wrapping_add(offset)) % pair_checks;
            let first = candidate / edge_count;
            let second = candidate % edge_count;
            if first >= second {
                continue;
            }
            let (a, b) = self.edges[first];
            let (c, d) = self.edges[second];
            if a == c || a == d || b == c || b == d {
                continue;
            }
            let [pa, pb, pc, pd] = [a, b, c, d].map(|index| self.nodes[index].position);
            if !Rect::from_two_pos(pa, pb).intersects(Rect::from_two_pos(pc, pd)) {
                continue;
            }
            let Some((t, u)) = crossing(pa, pb, pc, pd) else {
                continue;
            };
            let ab_normal = vec2(-(pb - pa).y, (pb - pa).x).normalized();
            let cd_normal = vec2(-(pd - pc).y, (pd - pc).x).normalized();
            // Move the endpoint closest to the other line across it. Moving
            // whole edges apart can leave a symmetric X locked in place.
            let candidates = [
                (a, b, c, d, u, cd_normal),
                (b, a, c, d, u, cd_normal),
                (c, d, a, b, t, ab_normal),
                (d, c, a, b, t, ab_normal),
            ];
            let &(node, partner, other_a, other_b, along, normal) = candidates
                .iter()
                .min_by(|left, right| {
                    let distance = |entry: &(usize, usize, usize, usize, f32, Vec2)| {
                        ((self.nodes[entry.0].position - self.nodes[entry.2].position).dot(entry.5))
                            .abs()
                    };
                    distance(left).total_cmp(&distance(right))
                })
                .unwrap();
            let origin = self.nodes[other_a].position;
            let distance = (self.nodes[node].position - origin).dot(normal).abs();
            let side = if (self.nodes[partner].position - origin).dot(normal) < 0.0 {
                -1.0
            } else {
                1.0
            };
            let force = normal * side * ((distance + 60.0) * 40.0).min(6000.0);
            forces[node] += force;
            forces[other_a] -= force * (1.0 - along);
            forces[other_b] -= force * along;
        }
    }
}

fn segment_rect_interval(start: Pos2, end: Pos2, rect: Rect) -> Option<(f32, f32)> {
    let delta = end - start;
    let mut entry = 0.0_f32;
    let mut exit = 1.0_f32;
    for (origin, direction, min, max) in [
        (start.x, delta.x, rect.min.x, rect.max.x),
        (start.y, delta.y, rect.min.y, rect.max.y),
    ] {
        if direction.abs() < 0.0001 {
            if origin < min || origin > max {
                return None;
            }
        } else {
            let a = (min - origin) / direction;
            let b = (max - origin) / direction;
            entry = entry.max(a.min(b));
            exit = exit.min(a.max(b));
            if entry > exit {
                return None;
            }
        }
    }
    Some((entry, exit))
}

/// Interior intersection of two segments; touching/shared endpoints are allowed.
fn crossing(a: Pos2, b: Pos2, c: Pos2, d: Pos2) -> Option<(f32, f32)> {
    let cross = |u: Vec2, v: Vec2| u.x * v.y - u.y * v.x;
    let ab = b - a;
    let cd = d - c;
    let denominator = cross(ab, cd);
    if denominator.abs() < 0.001 {
        return None;
    }
    let t = cross(c - a, cd) / denominator;
    let u = cross(c - a, ab) / denominator;
    (t > 0.0 && t < 1.0 && u > 0.0 && u < 1.0).then_some((t, u))
}

pub fn show(app_win: &mut AppWin, ui: &mut Ui) {
    let mut fit_requested = false;
    ui.horizontal_wrapped(|ui| {
        fit_requested = ui.button("Fit to view").clicked();
        ui.add_space(12.0);
        ui.scope(|ui| {
            let label = "Include flagged or banned players";
            let text = ui.painter().layout_no_wrap(
                label.into(), egui::TextStyle::Button.resolve(ui.style()), Color32::WHITE,
            );
            ui.set_max_width(text.size().x + 42.0);
            egui::containers::menu::menu_style(ui.style_mut());
            super::menu_item::checkbox(ui, &mut app_win.overview.include_flagged_players, label)
                .on_hover_text("Also show players without friend connections who have negative flags, Steam bans, or SourceBans history.");
        });
    });
    let (players, edges) = graph(&app_win.lobby, app_win.overview.include_flagged_players);
    let card_width = players
        .iter()
        .map(|player| required_card_width(ui, player))
        .fold(CARD_SIZE.x, f32::max);
    let state = &mut app_win.overview;
    let size_changed = state.card_size.x != card_width;
    state.card_size.x = card_width;
    state.sync(&players, edges, &app_win.lobby.lobby_id);
    if (fit_requested || size_changed) && !players.is_empty() {
        state.fit();
    }
    // The scene is a sublayer above this UI, so its nodes cover the help text.
    paint_canvas_help(ui, ui.available_rect_before_wrap());
    if players.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.weak(if state.include_flagged_players {
                "No friendships or flagged or banned players found on this server yet."
            } else {
                "No friendships found among players on this server yet."
            });
        });
        return;
    }

    let mut selected = None;
    let mut open_details = None;
    let mut dragged = None;
    let mut view = state.view;
    egui::Scene::new()
        .scroll_zooms(true)
        .zoom_range(0.05..=2.0)
        .show(ui, &mut view, |ui| {
            let painter = ui.painter().clone();
            let edge_stroke = Stroke::new(1.5_f32, theme::TEXT.gamma_multiply(0.65));
            for &(a, b) in &state.edges {
                let start = card_edge(
                    state.nodes[a].position,
                    state.nodes[b].position,
                    state.card_size,
                );
                let end = card_edge(
                    state.nodes[b].position,
                    state.nodes[a].position,
                    state.card_size,
                );
                let delta = end - start;
                if delta.dot(state.nodes[b].position - state.nodes[a].position) > 0.0 {
                    // Steam friendship is mutual: show arrowheads at both ends.
                    painter.line_segment([start, end], edge_stroke);
                    arrowhead(&painter, start, -delta.normalized(), edge_stroke);
                    arrowhead(&painter, end, delta.normalized(), edge_stroke);
                }
            }
            for (index, player) in players.iter().enumerate() {
                let node = &mut state.nodes[index];
                let rect = Rect::from_center_size(node.position, state.card_size);
                ui.expand_to_include_rect(rect.expand(30.0));
                let response = ui.interact(
                    rect,
                    ui.scope_id().with(player.steamid.to_u64()),
                    Sense::click_and_drag(),
                );
                if response.dragged() {
                    node.position += response.drag_delta();
                    dragged = Some(index);
                }
                if response.clicked() {
                    selected = Some(player.steamid);
                }
                if response.double_clicked() {
                    open_details = Some(player.steamid);
                }
                response
                    .clone()
                    .on_hover_text(format!("{}\nClick for details · drag to move", player.name));
                let border = match player.team {
                    Team::Blue => colors::TEAM_BLU_COLOR,
                    Team::Red => colors::TEAM_RED_COLOR,
                    Team::Spec => Color32::LIGHT_GRAY,
                    Team::Unknown => colors::GRAY,
                };
                painter.rect(
                    rect,
                    6.0,
                    theme::CONTROL,
                    Stroke::new(2.5_f32, border),
                    StrokeKind::Inside,
                );
                if app_win.selected_player == Some(player.steamid) || response.hovered() {
                    painter.rect_stroke(
                        rect.expand(3.0),
                        8.0,
                        Stroke::new(1.0_f32, theme::TEXT),
                        StrokeKind::Outside,
                    );
                }
                let avatar = Rect::from_min_size(rect.min + vec2(8.0, 9.0), Vec2::splat(36.0));
                if let Some(info) = &player.steam_info {
                    egui::Image::new(&info.avatar_thumb)
                        .corner_radius(3.0)
                        .paint_at(ui, avatar);
                } else {
                    painter.rect_filled(avatar, 3.0, theme::PANEL);
                    painter.text(
                        avatar.center(),
                        Align2::CENTER_CENTER,
                        "?",
                        FontId::proportional(18.0),
                        theme::TEXT,
                    );
                }
                paint_player_label(ui, player, rect);
            }
        });
    state.view = view;
    let dt = ui
        .input(|input| input.stable_dt)
        .clamp(1.0 / 240.0, 1.0 / 30.0);
    if state.step(dt, dragged) || dragged.is_some() {
        ui.ctx().request_repaint();
    }
    if let Some(steamid) = selected {
        app_win.set_selected_player(steamid);
    }
    if let Some(steamid) = open_details {
        app_win.open_player_details_window(steamid);
    }
}

fn paint_canvas_help(ui: &Ui, canvas: Rect) {
    let mut job = egui::text::LayoutJob::simple(
        "Drag players to rearrange\nDrag background to pan\nScroll to zoom".into(),
        FontId::proportional(11.0),
        theme::muted_text(),
        (canvas.width() - 24.0).max(1.0),
    );
    job.halign = egui::Align::RIGHT;
    let painter = ui
        .painter()
        .with_clip_rect(canvas.intersect(ui.clip_rect()));
    let text = painter.layout_job(job);
    painter.galley(
        canvas.max - Vec2::splat(12.0) - text.size(),
        text,
        theme::muted_text(),
    );
}

fn required_card_width(ui: &mut Ui, player: &Player) -> f32 {
    // Keep enough room for a readable name even when every badge is present.
    (53.0 + 96.0 + ui.spacing().item_spacing.x + player_badge_width(ui, player) + 8.0)
        .max(CARD_SIZE.x)
}

fn player_badge_width(ui: &mut Ui, player: &Player) -> f32 {
    let mut measure = ui.new_child(
        egui::UiBuilder::new()
            .id_salt(("overview_badges_measure", player.steamid.to_u64()))
            .max_rect(Rect::from_min_size(Pos2::ZERO, vec2(1000.0, CARD_SIZE.y)))
            .layout(egui::Layout::left_to_right(egui::Align::Center))
            .sizing_pass()
            .invisible(),
    );
    markings::add_reputation(&mut measure, player);
    measure.min_size().x
}

fn paint_player_label(ui: &mut Ui, player: &Player, card: Rect) {
    let content = Rect::from_min_max(
        egui::pos2(card.left() + 53.0, card.top()),
        egui::pos2(card.right() - 8.0, card.bottom()),
    );
    let badge_width = player_badge_width(ui, player);
    let gap = if badge_width > 0.0 {
        ui.spacing().item_spacing.x
    } else {
        0.0
    };
    let mut text = egui::text::LayoutJob::simple_singleline(
        player.name.clone(),
        FontId::proportional(13.0),
        theme::TEXT,
    );
    text.wrap.max_width = (content.width() - badge_width - gap).max(1.0);
    text.wrap.max_rows = 1;
    text.wrap.break_anywhere = true;
    let galley = ui.painter().layout_job(text);
    let badge_left = content.left() + galley.size().x + gap;
    ui.painter().galley(
        egui::pos2(content.left(), card.center().y - galley.size().y / 2.0),
        galley,
        theme::TEXT,
    );
    let mut badges = ui.new_child(
        egui::UiBuilder::new()
            .id_salt(("overview_badges", player.steamid.to_u64()))
            .max_rect(Rect::from_min_max(
                egui::pos2(badge_left, content.top()),
                content.max,
            ))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    markings::add_reputation(&mut badges, player);
}

fn card_edge(center: Pos2, toward: Pos2, card_size: Vec2) -> Pos2 {
    let direction = toward - center;
    let half = card_size * 0.5 + Vec2::splat(5.0);
    let scale = (half.x / direction.x.abs().max(0.001)).min(half.y / direction.y.abs().max(0.001));
    center + direction * scale
}

fn arrowhead(painter: &egui::Painter, tip: Pos2, direction: Vec2, stroke: Stroke) {
    let side = vec2(-direction.y, direction.x) * 4.0;
    let base = tip - direction * 8.0;
    painter.line_segment([base + side, tip], stroke);
    painter.line_segment([base - side, tip], stroke);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tf2::lobby::friendships::Friendships;

    fn lobby() -> Lobby {
        let mut lobby = Lobby::default();
        lobby.players = (1..=5)
            .map(|id| Player {
                steamid: SteamID::from_u64(id),
                name: format!("Player {id}"),
                friends: (id == 1).then(|| [SteamID::from_u64(2), SteamID::from_u64(99)].into()),
                ..Default::default()
            })
            .collect();
        lobby.friendships = Friendships::from_lobby(&lobby);
        lobby
    }

    #[test]
    fn only_current_players_with_friends_are_shown_once_including_private_profiles() {
        let mut lobby = lobby();
        let (players, edges) = graph(&lobby, false);
        assert_eq!(
            players
                .iter()
                .map(|p| p.steamid.to_u64())
                .collect::<Vec<_>>(),
            [1, 2]
        );
        assert_eq!(edges, [(0, 1)]);
        assert!(players[1].friends.is_none());
        lobby.players.remove(1);
        // Even if the friendship cache hasn't refreshed, departed players disappear.
        assert!(graph(&lobby, false).0.is_empty());
    }

    fn bans(vac: bool, game: bool) -> crate::tf2::steamapi::SteamPlayerBan {
        crate::tf2::steamapi::SteamPlayerBan {
            steamid: SteamID::from_u64(3),
            community_banned: false,
            vac_banned: vac,
            number_of_vac_bans: u32::from(vac),
            days_since_last_ban: 10,
            number_of_game_bans: u32::from(game),
        }
    }

    fn reputation(warning: bool) -> crate::reputation::Reputation {
        let mut record = crate::ban_data::tests::ban();
        record.reason = Some(if warning { "Cheating" } else { "Spam" }.into());
        crate::reputation::Reputation {
            steamid: SteamID::from_u64(5),
            complete: true,
            source_bans: vec![crate::reputation::SourceBan {
                source: "Test".into(),
                record,
            }],
        }
    }

    #[test]
    fn optional_filter_adds_isolated_flagged_and_banned_players_without_false_friendships() {
        use crate::tf2bd::models::{PlayerAttribute, PlayerInfo};
        let mut lobby = lobby();
        lobby.players[2].steam_bans = Some(bans(true, false));
        lobby.players[3].steam_bans = Some(bans(false, true));
        lobby.players[4].reputation = Some(reputation(false));
        for (id, flag) in [(6, PlayerAttribute::Cool), (7, PlayerAttribute::Cheater)] {
            lobby.players.push(Player {
                steamid: SteamID::from_u64(id),
                player_info: Some(PlayerInfo {
                    attributes: vec![flag],
                    ..Default::default()
                }),
                ..Default::default()
            });
        }
        assert_eq!(graph(&lobby, false).0.len(), 2);
        let (players, edges) = graph(&lobby, true);
        assert_eq!(
            players
                .iter()
                .map(|p| p.steamid.to_u64())
                .collect::<Vec<_>>(),
            [1, 2, 3, 4, 5, 7]
        );
        assert_eq!(edges, [(0, 1)]);
        let mut state = OverviewState {
            include_flagged_players: true,
            ..Default::default()
        };
        state.sync(&players, edges, "new session");
        assert!(state.include_flagged_players);
    }

    #[test]
    fn shared_ban_badges_follow_name_only_when_present_and_fit_inside_card() {
        let ctx = egui::Context::default();
        theme::install(&ctx);
        for flags in 0..8 {
            for warning in [false, true] {
                let player = Player {
                    steamid: SteamID::from_u64(1),
                    name: "A very long player name that needs to truncate".into(),
                    steam_bans: Some(bans(flags & 1 != 0, flags & 2 != 0)),
                    reputation: (flags & 4 != 0).then(|| reputation(warning)),
                    ..Default::default()
                };
                let card = Rect::from_min_size(egui::pos2(20.0, 20.0), CARD_SIZE);
                let output = super::super::tests::run_ui(
                    &ctx,
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(500.0, 200.0))),
                        ..Default::default()
                    },
                    |root| {
                        egui::CentralPanel::default()
                            .show(root, |ui| paint_player_label(ui, &player, card));
                    },
                );
                let texts: Vec<_> = output
                    .shapes
                    .iter()
                    .filter_map(|s| match &s.shape {
                        egui::Shape::Text(t) => Some(t),
                        _ => None,
                    })
                    .collect();
                let badges: Vec<_> = texts
                    .iter()
                    .filter(|t| ["VAC", "GB", "SB"].contains(&t.galley.text()))
                    .collect();
                assert_eq!(
                    badges.iter().map(|t| t.galley.text()).collect::<Vec<_>>(),
                    [
                        (flags & 1 != 0, "VAC"),
                        (flags & 2 != 0, "GB"),
                        (flags & 4 != 0, "SB")
                    ]
                    .into_iter()
                    .filter_map(|(show, label)| show.then_some(label))
                    .collect::<Vec<_>>()
                );
                let name = texts[0];
                if flags == 0 {
                    assert!(name.galley.size().x > 150.0);
                }
                let mut right = name.pos.x + name.galley.size().x;
                for badge in badges {
                    assert!(badge.pos.x > right, "Name and badges must not overlap");
                    right = badge.pos.x + badge.galley.size().x;
                    assert!(
                        right <= card.right() - 8.0,
                        "Badges must fit inside the card"
                    );
                }
            }
        }
    }

    #[test]
    fn all_markings_share_grid_badges_and_leave_room_for_name() {
        use crate::tf2bd::models::{PlayerAttribute::*, PlayerInfo};
        let ctx = egui::Context::default();
        theme::install(&ctx);
        let player = Player {
            steamid: SteamID::from_u64(1),
            name: "A long player name with every marking".into(),
            steam_bans: Some(bans(true, true)),
            reputation: Some(reputation(true)),
            player_info: Some(PlayerInfo {
                attributes: vec![Cool, Cheater, Bot, Suspicious, Toxic, Exploiter],
                ..Default::default()
            }),
            ..Default::default()
        };
        let mut card = Rect::NOTHING;
        let output = super::super::tests::run_ui(
            &ctx,
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 200.0))),
                ..Default::default()
            },
            |root| {
                egui::CentralPanel::default().show(root, |ui| {
                    let width = required_card_width(ui, &player);
                    card = Rect::from_min_size(egui::pos2(20.0, 20.0), vec2(width, CARD_SIZE.y));
                    paint_player_label(ui, &player, card);
                });
            },
        );
        assert!(card.width() > CARD_SIZE.x);
        let texts: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::Text(t) => Some(t),
                _ => None,
            })
            .collect();
        assert_eq!(
            texts
                .iter()
                .skip(1)
                .map(|t| t.galley.text())
                .collect::<Vec<_>>(),
            ["VAC", "GB", "★", "C", "B", "S", "T", "E", "SB"]
        );
        assert!(texts[0].galley.size().x >= 80.0);
        for pair in texts.windows(2) {
            assert!(pair[0].pos.x + pair[0].galley.size().x < pair[1].pos.x);
        }
        assert!(
            texts.last().unwrap().pos.x + texts.last().unwrap().galley.size().x
                < card.right() - 8.0
        );
    }

    #[test]
    fn canvas_help_stays_in_bottom_right_and_is_painted_below_nodes() {
        let mut app = crate::models::player_profile::test_app();
        app.lobby = lobby();
        let ctx = egui::Context::default();
        theme::install(&ctx);
        let mut previous_corner = None;
        for frame in 0..2 {
            if frame == 1 {
                app.overview.view = app.overview.view.translate(vec2(90.0, -70.0));
            }
            let mut expected = Pos2::ZERO;
            let output = super::super::tests::run_ui(
                &ctx,
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 700.0))),
                    ..Default::default()
                },
                |root| {
                    egui::CentralPanel::default().show(root, |ui| {
                        expected = ui.max_rect().max - Vec2::splat(12.0);
                        show(&mut app, ui);
                    });
                },
            );
            let (help_index, help) = output
                .shapes
                .iter()
                .enumerate()
                .find_map(|(index, s)| match &s.shape {
                    egui::Shape::Text(t) if t.galley.text().contains("Scroll to zoom") => {
                        Some((index, t))
                    }
                    _ => None,
                })
                .unwrap();
            let corner = help.pos + help.galley.size();
            assert!(corner.distance(expected) < 0.1);
            if let Some(previous) = previous_corner {
                assert!(corner.distance(previous) < 0.1);
            }
            previous_corner = Some(corner);
            let node_index = output
                .shapes
                .iter()
                .position(
                    |s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == "Player 1"),
                )
                .unwrap();
            assert!(
                help_index < node_index,
                "Scene nodes must be painted over the fixed help text"
            );
        }
    }

    #[test]
    fn spring_layout_stays_finite_separates_cards_and_preserves_positions_on_refresh() {
        let lobby = lobby();
        let (players, edges) = graph(&lobby, false);
        let mut state = OverviewState::default();
        state.sync(&players, edges.clone(), "server");
        for _ in 0..1200 {
            state.step(1.0 / 60.0, None);
        }
        assert!(state.nodes.iter().all(|node| node.position.is_finite()));
        let a = Rect::from_center_size(state.nodes[0].position, CARD_SIZE);
        let b = Rect::from_center_size(state.nodes[1].position, CARD_SIZE);
        assert!(!a.intersects(b));
        let positions: Vec<_> = state.nodes.iter().map(|node| node.position).collect();
        state.sync(&players, edges, "server");
        assert_eq!(
            positions,
            state
                .nodes
                .iter()
                .map(|node| node.position)
                .collect::<Vec<_>>()
        );
        state.sync(&[], vec![], "server");
        assert!(state.nodes.is_empty());
        assert!(state.edges.is_empty());
    }

    #[test]
    fn dense_friend_groups_remain_separated_and_finite() {
        let players: Vec<_> = (0..24)
            .map(|id| Player {
                steamid: SteamID::from_u64(id),
                ..Default::default()
            })
            .collect();
        let refs: Vec<_> = players.iter().collect();
        let edges = (0..24)
            .flat_map(|a| {
                (a + 1..24)
                    .filter(move |b| a / 6 == b / 6)
                    .map(move |b| (a, b))
            })
            .collect();
        let mut state = OverviewState::default();
        state.sync(&refs, edges, "dense");
        for _ in 0..1800 {
            state.step(1.0 / 60.0, None);
        }
        for (a, node) in state.nodes.iter().enumerate() {
            assert!(node.position.is_finite());
            for other in &state.nodes[a + 1..] {
                assert!(
                    !Rect::from_center_size(node.position, CARD_SIZE)
                        .intersects(Rect::from_center_size(other.position, CARD_SIZE))
                );
            }
        }
    }

    fn positioned_graph(positions: &[Pos2], edges: Vec<(usize, usize)>) -> OverviewState {
        OverviewState {
            nodes: positions
                .iter()
                .enumerate()
                .map(|(id, &position)| Node {
                    steamid: SteamID::from_u64(id as u64),
                    position,
                    velocity: Vec2::ZERO,
                })
                .collect(),
            edges,
            ..Default::default()
        }
    }

    #[test]
    fn layout_untangles_crossing_edges_and_clears_obstructing_cards() {
        let mut crossed = positioned_graph(
            &[
                egui::pos2(-220.0, -140.0),
                egui::pos2(220.0, 140.0),
                egui::pos2(-220.0, 140.0),
                egui::pos2(220.0, -140.0),
            ],
            vec![(0, 1), (2, 3)],
        );
        assert!(
            crossing(
                crossed.nodes[0].position,
                crossed.nodes[1].position,
                crossed.nodes[2].position,
                crossed.nodes[3].position
            )
            .is_some()
        );
        let mut blocked = positioned_graph(
            &[egui::pos2(-260.0, 0.0), egui::pos2(260.0, 0.0), Pos2::ZERO],
            vec![(0, 1)],
        );
        for _ in 0..900 {
            crossed.step(1.0 / 60.0, None);
            blocked.step(1.0 / 60.0, None);
        }
        assert!(
            crossing(
                crossed.nodes[0].position,
                crossed.nodes[1].position,
                crossed.nodes[2].position,
                crossed.nodes[3].position
            )
            .is_none()
        );
        assert!(
            segment_rect_interval(
                blocked.nodes[0].position,
                blocked.nodes[1].position,
                Rect::from_center_size(blocked.nodes[2].position, CARD_SIZE)
            )
            .is_none()
        );
    }

    #[test]
    fn incident_edges_do_not_penalize_their_own_cards_or_shared_endpoints() {
        let state = positioned_graph(
            &[
                Pos2::ZERO,
                egui::pos2(300.0, -100.0),
                egui::pos2(300.0, 100.0),
            ],
            vec![(0, 1), (0, 2)],
        );
        let mut forces = vec![Vec2::ZERO; 3];
        state.add_avoidance_forces(&mut forces);
        assert!(forces.iter().all(|force| *force == Vec2::ZERO));
    }

    #[test]
    fn avoidance_cost_for_sparse_and_dense_servers() {
        for (count, dense) in [(32, false), (32, true), (64, true)] {
            let positions: Vec<_> = (0..count)
                .map(|index| {
                    let angle = index as f32 * std::f32::consts::TAU / count as f32;
                    Pos2::ZERO + vec2(angle.cos(), angle.sin()) * 700.0
                })
                .collect();
            let edges = (0..count)
                .flat_map(|a| {
                    (a + 1..count)
                        .filter(move |&b| dense || b - a <= 2)
                        .map(move |b| (a, b))
                })
                .collect();
            let mut state = positioned_graph(&positions, edges);
            let mut forces = vec![Vec2::ZERO; count];
            let start = std::time::Instant::now();
            for _ in 0..300 {
                forces.fill(Vec2::ZERO);
                state.add_avoidance_forces(&mut forces);
                state.avoidance_cursor += AVOIDANCE_BUDGET;
                std::hint::black_box(&forces);
            }
            eprintln!(
                "{count} players, {} edges: {:.3} ms/avoidance update",
                state.edges.len(),
                start.elapsed().as_secs_f64() * 1000.0 / 300.0
            );
            assert!(forces.iter().all(|force| force.is_finite()));
        }
    }

    #[test]
    fn canvas_renders_and_selects_players_in_transformed_scene() {
        use crate::{appbus::AppBus, models::app_settings::AppSettings};
        use diesel::{
            SqliteConnection,
            r2d2::{ConnectionManager, Pool},
        };
        use std::sync::{Arc, Mutex};

        let settings = AppSettings::default();
        let bus = Arc::new(Mutex::new(AppBus::new(settings.self_steamid64)));
        let pool = Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(":memory:"))
            .unwrap();
        let mut app = AppWin::new(&settings, &bus, Arc::new(pool));
        app.lobby = lobby();
        let ctx = egui::Context::default();
        theme::install(&ctx);
        let mut point = Pos2::ZERO;
        for frame in 0..4 {
            let events = match frame {
                2 | 3 => vec![
                    egui::Event::PointerMoved(point),
                    egui::Event::PointerButton {
                        pos: point,
                        button: egui::PointerButton::Primary,
                        pressed: frame == 2,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                _ => vec![],
            };
            let output = super::super::tests::run_ui(
                &ctx,
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 700.0))),
                    events,
                    ..Default::default()
                },
                |root_ui| {
                    let ctx = &root_ui.ctx().clone();
                    egui::CentralPanel::default().show(root_ui, |ui| {
                        let layer = egui::LayerId::new(
                            ui.layer_id().order,
                            ui.scope_id().with("scene_area"),
                        );
                        show(&mut app, ui);
                        point = ctx.layer_transform_to_global(layer).unwrap()
                            * app.overview.nodes[0].position;
                    });
                },
            );
            for primitive in ctx.tessellate(output.shapes, output.pixels_per_point) {
                if let egui::epaint::Primitive::Mesh(mesh) = primitive.primitive {
                    assert!(mesh.vertices.iter().all(|vertex| vertex.pos.is_finite()));
                }
            }
        }
        assert_eq!(app.selected_player, Some(SteamID::from_u64(1)));
    }
}
