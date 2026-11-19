//! Headless input tests exercise egui hit testing and emitted native commands.
use super::{menu_item, theme, title_bar};
use eframe::egui::{self, Event, Pos2, Rect, ViewportCommand};

/// Inspect shapes and commands without a renderer consuming the texture updates.
pub(super) fn run_ui(
    ctx: &egui::Context,
    input: egui::RawInput,
    ui: impl FnMut(&mut egui::Ui),
) -> egui::FullOutput {
    let mut output = ctx.run_ui(input, ui);
    output.textures_delta.clear();
    output
}

fn input(events: Vec<Event>, scale: f32, maximized: bool, fullscreen: bool) -> egui::RawInput {
    let mut input = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 600.0))),
        events,
        ..Default::default()
    };
    let viewport = input.viewports.get_mut(&egui::ViewportId::ROOT).unwrap();
    viewport.native_pixels_per_point = Some(scale);
    viewport.maximized = Some(maximized);
    viewport.fullscreen = Some(fullscreen);
    input
}

fn pointer(pos: Pos2, pressed: bool) -> Vec<Event> {
    vec![
        Event::PointerMoved(pos),
        Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        },
    ]
}

fn commands(output: &egui::FullOutput) -> &[ViewportCommand] {
    &output.viewport_output[&egui::ViewportId::ROOT].commands
}

#[test]
fn tab_indicator_moves_along_the_baseline_when_switching_views() {
    use super::tabs::{self, Tab};

    let ctx = egui::Context::default();
    theme::install(&ctx);
    let mut tab = Tab::Game;
    let mut frame = |events| {
        let output = run_ui(&ctx, input(events, 1.0, false, false), |root| {
            egui::CentralPanel::default().show(root, |ui| tabs::add_tab_bar(&mut tab, ui));
        });
        (output, tab)
    };
    let label_center = |output: &egui::FullOutput, label| {
        output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.text() == label => {
                Some(text.pos + text.galley.size() / 2.0)
            }
            _ => None,
        })
    };
    let indicator = |output: &egui::FullOutput, label| {
        let mut baseline = None;
        let mut indicator = None;
        for shape in &output.shapes {
            if let egui::Shape::LineSegment { points, stroke } = &shape.shape {
                if points[1].x - points[0].x > 700.0 {
                    baseline = Some(points[0].y);
                } else if stroke.width == 2.0 {
                    indicator = Some(*points);
                }
            }
        }
        let [left, right] = indicator.expect("The selected tab has an underline");
        assert_eq!(Some(left.y), baseline);
        let center = label_center(output, label).unwrap();
        assert!(left.x < center.x && center.x < right.x);
    };

    let (output, active) = frame(vec![]);
    assert_eq!(active, Tab::Game);
    indicator(&output, "Scoreboard");
    let overview = label_center(&output, "Overview").unwrap();
    frame(pointer(overview, true));
    let (output, active) = frame(pointer(overview, false));
    assert_eq!(active, Tab::Overview);
    indicator(&output, "Overview");
}

fn title_frame(
    ctx: &egui::Context,
    events: Vec<Event>,
    maximized: bool,
) -> (egui::FullOutput, [Rect; 3]) {
    let mut buttons = [Rect::NOTHING; 3];
    let output = run_ui(ctx, input(events, 1.0, maximized, false), |root_ui| {
        egui::Panel::top("test_title")
            .frame(egui::Frame::new().inner_margin(egui::Margin {
                left: 8,
                right: 0,
                top: 2,
                bottom: 2,
            }))
            .show(root_ui, |ui| {
                theme::menu_bar().ui(ui, |ui| {
                    ui.menu_button("File", |_| {});
                    buttons = title_bar::contents(ui, "TF2 Monitor").map(|r| r.rect);
                });
            });
    });
    (output, buttons)
}

#[test]
fn title_controls_emit_native_commands_and_meet_right_edge() {
    for (index, maximized) in [(0, false), (1, false), (1, true), (2, false)] {
        let ctx = egui::Context::default();
        theme::install(&ctx);
        let (output, rects) = title_frame(&ctx, vec![], maximized);
        let title = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == "TF2 Monitor" => Some(text),
                _ => None,
            })
            .expect("The window title should be visible");
        assert!((title.pos.x + title.galley.size().x / 2.0 - 400.0).abs() < 1.0);
        assert!(
            commands(&output)
                .iter()
                .any(|c| matches!(c, ViewportCommand::Title(t) if t == "TF2 Monitor"))
        );
        assert!((rects[2].right() - 800.0).abs() < 0.5, "{rects:?}");
        assert!((rects[2].width() - 42.0).abs() < 0.1);
        title_frame(&ctx, pointer(rects[index].center(), true), maximized);
        let (output, _) = title_frame(&ctx, pointer(rects[index].center(), false), maximized);
        assert!(
            commands(&output).iter().any(|c| match (index, c) {
                (0, ViewportCommand::Minimized(true)) | (2, ViewportCommand::Close) => true,
                (1, ViewportCommand::Maximized(v)) => *v != maximized,
                _ => false,
            }),
            "{:?}",
            commands(&output)
        );
    }
}

#[test]
fn title_drag_and_double_click_use_native_commands() {
    let ctx = egui::Context::default();
    theme::install(&ctx);
    let (_, rects) = title_frame(&ctx, vec![], false);
    let pos = rects[0].center() - egui::vec2(120.0, 0.0);
    let (output, _) = title_frame(&ctx, pointer(pos, true), false);
    assert!(
        !commands(&output)
            .iter()
            .any(|c| matches!(c, ViewportCommand::StartDrag))
    );
    let (output, _) = title_frame(
        &ctx,
        vec![Event::PointerMoved(pos + egui::vec2(20.0, 0.0))],
        false,
    );
    assert!(
        commands(&output)
            .iter()
            .any(|c| matches!(c, ViewportCommand::StartDrag))
    );
    for maximized in [false, true] {
        let ctx = egui::Context::default();
        theme::install(&ctx);
        title_frame(&ctx, vec![], maximized);
        for click in 0..2 {
            let (pressed, _) = title_frame(&ctx, pointer(pos, true), maximized);
            let (released, _) = title_frame(&ctx, pointer(pos, false), maximized);
            for output in [&pressed, &released] {
                assert!(
                    !commands(output)
                        .iter()
                        .any(|c| matches!(c, ViewportCommand::StartDrag))
                );
            }
            assert_eq!(
                commands(&released)
                    .iter()
                    .any(|c| matches!(c, ViewportCommand::Maximized(value) if *value != maximized)),
                click == 1,
            );
        }
    }
}

#[test]
fn all_resize_directions_and_blocking_states() {
    use egui::ResizeDirection::*;
    for (pos, direction, cursor) in [
        (
            egui::pos2(2.0, 2.0),
            NorthWest,
            egui::CursorIcon::ResizeNwSe,
        ),
        (
            egui::pos2(798.0, 2.0),
            NorthEast,
            egui::CursorIcon::ResizeNeSw,
        ),
        (
            egui::pos2(2.0, 598.0),
            SouthWest,
            egui::CursorIcon::ResizeNeSw,
        ),
        (
            egui::pos2(798.0, 598.0),
            SouthEast,
            egui::CursorIcon::ResizeNwSe,
        ),
        (
            egui::pos2(400.0, 2.0),
            North,
            egui::CursorIcon::ResizeVertical,
        ),
        (
            egui::pos2(400.0, 598.0),
            South,
            egui::CursorIcon::ResizeVertical,
        ),
        (
            egui::pos2(2.0, 300.0),
            West,
            egui::CursorIcon::ResizeHorizontal,
        ),
        (
            egui::pos2(798.0, 300.0),
            East,
            egui::CursorIcon::ResizeHorizontal,
        ),
    ] {
        for (maximized, fullscreen, modal) in [
            (false, false, false),
            (true, false, false),
            (false, true, false),
            (false, false, true),
        ] {
            let ctx = egui::Context::default();
            theme::install(&ctx);
            let frame = |events| {
                run_ui(&ctx, input(events, 1.0, maximized, fullscreen), |root_ui| {
                    let ctx = &root_ui.ctx().clone();
                    egui::CentralPanel::default().show(root_ui, |ui| {
                        ui.label("Content");
                    });
                    if modal {
                        egui::Modal::new(egui::Id::unique("modal")).show(ctx, |ui| {
                            ui.label("Blocking dialog");
                        });
                    }
                    title_bar::resize_edges(ctx);
                })
            };
            frame(vec![Event::PointerMoved(pos)]);
            frame(vec![Event::PointerMoved(pos)]);
            let output = frame(pointer(pos, true));
            let resizing = commands(&output)
                .iter()
                .any(|c| matches!(c,ViewportCommand::BeginResize(d) if *d == direction));
            assert_eq!(
                resizing,
                !maximized && !fullscreen && !modal,
                "{direction:?}, max={maximized}, full={fullscreen}, modal={modal}: {:?}",
                commands(&output)
            );
            if resizing {
                assert_eq!(output.platform_output.cursor_icon, cursor);
            }
        }
    }
}

#[test]
fn separators_and_font_fallback_at_fractional_dpi() {
    for scale in [1.0, 1.25, 1.5, 2.0] {
        let ctx = egui::Context::default();
        theme::install(&ctx);
        let output = run_ui(&ctx, input(vec![], scale, false, false), |root_ui| {
            let ctx = &root_ui.ctx().clone();
            egui::CentralPanel::default().show(root_ui, |ui| {
                ctx.fonts_mut(|fonts| {
                    for font in [
                        egui::FontId::proportional(13.0),
                        theme::heading_font(),
                        theme::shortcut_font(),
                    ] {
                        assert!(fonts.has_glyphs(&font, "TF2 Åäö 0123 × − …"));
                    }
                });
                for edge in [
                    theme::PanelEdge::Top,
                    theme::PanelEdge::Bottom,
                    theme::PanelEdge::Left,
                ] {
                    let response = ui.interact(
                        Rect::from_min_max(egui::pos2(0.3, 0.3), egui::pos2(700.7, 500.7)),
                        egui::Id::unique("panel"),
                        egui::Sense::hover(),
                    );
                    theme::panel_separator(ctx, &response, edge);
                }
            });
        });
        let lines: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::Rect(r) if r.fill == theme::BORDER && r.stroke.is_empty() => {
                    Some(r.rect * ctx.pixels_per_point())
                }
                _ => None,
            })
            .collect();
        assert_eq!(lines.len(), 3);
        for rect in lines {
            assert!((rect.width().min(rect.height()) - 1.0).abs() < 0.001);
            for c in [rect.min.x, rect.min.y, rect.max.x, rect.max.y] {
                assert!((c - c.round()).abs() < 0.001);
            }
        }
    }
}

#[test]
fn menus_open_on_click_switch_on_hover_and_close_on_escape_or_outside() {
    let ctx = egui::Context::default();
    theme::install(&ctx);
    let mut heads = [Rect::NOTHING; 2];
    let mut visible = [false; 2];
    let mut frame = |events| {
        visible.fill(false);
        let output = run_ui(&ctx, input(events, 1.0, false, false), |root_ui| {
            egui::Panel::top("menus").show(root_ui, |ui| {
                theme::menu_bar().ui(ui, |ui| {
                    for (index, label) in ["File", "View"].iter().enumerate() {
                        heads[index] = theme::menu_header(ui, label, |ui| {
                            visible[index] = true;
                            theme::menu_contents(ui);
                            menu_item::command(ui, "Command");
                        })
                        .response
                        .rect;
                    }
                });
            });
        });
        (output, heads, visible)
    };
    let (_, heads, _) = frame(vec![]);
    let (_, _, visible) = frame(vec![Event::PointerMoved(heads[0].center())]);
    assert_eq!(visible, [false, false]);
    frame(pointer(heads[0].center(), true));
    frame(pointer(heads[0].center(), false));
    let (_, _, visible) = frame(vec![]);
    assert_eq!(visible, [true, false]);
    frame(vec![Event::PointerMoved(heads[1].center())]);
    let (_, _, visible) = frame(vec![]);
    assert_eq!(visible, [false, true]);
    frame(vec![Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }]);
    let (_, _, visible) = frame(vec![]);
    assert_eq!(visible, [false, false]);
    frame(pointer(heads[0].center(), true));
    frame(pointer(heads[0].center(), false));
    frame(pointer(egui::pos2(700.0, 500.0), true));
    frame(pointer(egui::pos2(700.0, 500.0), false));
    let (_, _, visible) = frame(vec![]);
    assert_eq!(visible, [false, false]);
}

#[test]
fn menu_rows_share_text_column_and_checkbox_has_one_click_target() {
    let ctx = egui::Context::default();
    theme::install(&ctx);
    let mut checked = false;
    let mut rect = Rect::NOTHING;
    let mut frame = |events, enabled| {
        let output = run_ui(&ctx, input(events, 1.0, false, false), |root_ui| {
            egui::CentralPanel::default().show(root_ui, |ui| {
                ui.set_max_width(350.0);
                ui.add_enabled_ui(enabled, |ui| {
                    menu_item::command_with_keys(
                        ui,
                        "Command",
                        &[super::shortcut::command_label(), "Q"],
                    );
                    rect = menu_item::checkbox(ui, &mut checked, "Toggle").rect;
                });
            });
        });
        (output, rect, checked)
    };
    let (output, rect, _) = frame(vec![], true);
    let positions: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Text(t) if ["Command", "Toggle"].contains(&t.galley.text()) => {
                Some(t.pos.x)
            }
            _ => None,
        })
        .collect();
    assert_eq!(positions.len(), 2);
    assert!((positions[0] - positions[1]).abs() < 0.01);
    for (pos, expected) in [
        (rect.left_center() + egui::vec2(12.0, 0.0), true),
        (rect.right_center() - egui::vec2(5.0, 0.0), false),
    ] {
        frame(pointer(pos, true), true);
        let (_, _, checked) = frame(pointer(pos, false), true);
        assert_eq!(checked, expected);
    }
    frame(vec![], false);
    frame(pointer(rect.center(), true), false);
    let (_, _, checked) = frame(pointer(rect.center(), false), false);
    assert!(!checked, "Disabled rows must not toggle");
}

#[test]
fn native_and_menu_checkboxes_share_the_theme_frame() {
    let ctx = egui::Context::default();
    theme::install(&ctx);
    let output = run_ui(&ctx, input(vec![], 1.0, false, false), |root_ui| {
        egui::CentralPanel::default().show(root_ui, |ui| {
            ui.checkbox(&mut true, "Dialog checkbox");
            menu_item::checkbox(ui, &mut true, "Menu checkbox");
        });
    });
    let frames: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Rect(rect) if rect.rect.size() == egui::Vec2::splat(theme::CHECKBOX_SIZE) => {
                Some((rect.fill, rect.stroke, rect.corner_radius))
            }
            _ => None,
        })
        .collect();
    let frame = theme::checkbox_frame(egui::Color32::WHITE);
    assert_eq!(frames, [(frame.fill, frame.stroke, frame.corner_radius); 2]);
}

#[test]
fn colored_checkboxes_share_badge_colors_in_dialogs_and_menus() {
    use super::colors::TF2BD_BADGE_COLORS;
    use crate::tf2bd::models::PlayerAttribute::{Bot, Cheater};

    let ctx = egui::Context::default();
    theme::install(&ctx);
    let output = run_ui(&ctx, input(vec![], 1.0, false, false), |root_ui| {
        egui::CentralPanel::default().show(root_ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                theme::flag_checkbox(ui, &mut true, Cheater);
                theme::flag_checkbox(ui, &mut false, Bot);
            });
            menu_item::flag_checkbox(ui, &mut true, "Cheater", Cheater);
            menu_item::flag_checkbox(ui, &mut false, "Bot", Bot);
            menu_item::colored_checkbox(ui, &mut true, "TF2BD", TF2BD_BADGE_COLORS);
        });
    });
    let colors: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Rect(rect)
                if rect.rect.size() == egui::Vec2::splat(theme::COLORED_CHECKBOX_SIZE) =>
            {
                Some((rect.stroke.color, rect.fill))
            }
            _ => None,
        })
        .collect();
    let [cheater, bot] = [Cheater, Bot].map(super::colors::color_for_flag);
    let (tf2bd_checkmark, tf2bd_background) = TF2BD_BADGE_COLORS;
    assert_eq!(
        colors,
        [
            cheater,
            bot,
            cheater,
            bot,
            (egui::Color32::BLACK, tf2bd_background)
        ]
    );
    assert!(output.shapes.iter().any(|shape| {
        matches!(&shape.shape, egui::Shape::Path(path)
            if path.points.len() == 3 && path.stroke == egui::epaint::PathStroke::new(
                theme::CHECKMARK_STROKE.width, tf2bd_checkmark))
    }));
}

#[test]
fn shortcut_commands_respect_editor_and_modal_focus() {
    for (editor, modal) in [(false, false), (true, false), (false, true)] {
        let ctx = egui::Context::default();
        theme::install(&ctx);
        let mut text = String::new();
        let mut frame = |events| {
            let mut consumed = false;
            let _ = run_ui(&ctx, input(events, 1.0, false, false), |root_ui| {
                let ctx = &root_ui.ctx().clone();
                egui::CentralPanel::default().show(root_ui, |ui| {
                    if editor {
                        ui.text_edit_singleline(&mut text).request_focus();
                    }
                });
                if modal {
                    egui::Modal::new(egui::Id::unique("modal")).show(ctx, |ui| {
                        ui.label("Dialog");
                    });
                }
                consumed = super::shortcut::consume_command(ctx, egui::Key::Q);
            });
            consumed
        };
        frame(vec![]);
        let consumed = frame(vec![Event::Key {
            key: egui::Key::Q,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::COMMAND,
        }]);
        assert_eq!(consumed, !editor && !modal);
    }
}

#[test]
fn menu_hints_stay_outside_popup_and_flip_at_screen_edge() {
    use theme::MenuTooltip;
    for x in [30.0, 510.0] {
        let ctx = egui::Context::default();
        theme::install(&ctx);
        ctx.memory_mut(|m| m.set_everything_is_visible(true));
        let mut menu = Rect::NOTHING;
        let mut item = egui::Id::NULL;
        for _ in 0..3 {
            let _ = run_ui(&ctx, input(vec![], 1.0, false, false), |root_ui| {
                let ctx = &root_ui.ctx().clone();
                let area = egui::Area::new(egui::Id::unique("test_menu")).fixed_pos(egui::pos2(x,50.0)).show(ctx, |ui| {
                    ui.set_width(220.0);
                    item = menu_item::command(ui,"Command").menu_tooltip("An explanatory menu hint with enough text to wrap, without covering other menu rows.").id;
                });
                menu = area.response.rect;
            });
        }
        let hint = ctx
            .memory(|m| m.area_rect(egui::Tooltip::tooltip_id(item, 0)))
            .unwrap();
        if x < 100.0 {
            assert!(hint.left() >= menu.right() + 10.0, "{hint:?} vs {menu:?}");
        } else {
            assert!(hint.right() <= menu.left() - 10.0, "{hint:?} vs {menu:?}");
        }
    }
}

#[test]
fn long_title_keeps_controls_visible_at_supported_scales() {
    for scale in [1.0, 1.25, 1.5, 2.0] {
        let ctx = egui::Context::default();
        theme::install(&ctx);
        let _ = run_ui(&ctx, input(vec![], scale, false, false), |root_ui| {
            egui::Panel::top("title")
                .frame(egui::Frame::new().inner_margin(egui::Margin {left:8,right:0,top:2,bottom:2}))
                .show(root_ui, |ui| {
                    theme::menu_bar().ui(ui, |ui| {
                        for label in ["File","View","Actions","Tools"] { theme::menu_header(ui,label, |_| {}); }
                        let controls = title_bar::contents(ui, "A very long application title that must truncate without moving or hiding the native window controls");
                        assert!((controls[2].rect.right()-800.0).abs()<1.0);
                        for control in controls { assert!((control.rect.width()-42.0).abs()<1.0); }
                    });
                });
        });
    }
}

#[test]
fn file_menu_opens_the_markings_import_dialog_with_merge_selected() {
    let ctx = egui::Context::default();
    theme::install(&ctx);
    let mut app = crate::models::player_profile::test_app();
    let mut frame = |events| {
        run_ui(&ctx, input(events, 1.0, false, false), |root| {
            egui::CentralPanel::default().show(root, |ui| {
                super::top_menu::add_top_menu(ui, &mut app);
            });
            super::marking_transfer::show(&mut app, &ctx);
        })
    };
    let label_center = |output: &egui::FullOutput, label| {
        output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.text() == label => {
                Some(text.pos + text.galley.size() / 2.0)
            }
            _ => None,
        })
    };

    let file = label_center(&frame(vec![]), "File").unwrap();
    frame(pointer(file, true));
    frame(pointer(file, false));
    let menu = frame(vec![]);
    assert!(label_center(&menu, "Export Player Markings...").is_some());
    let import = label_center(&menu, "Import Player Markings...").unwrap();
    frame(pointer(import, true));
    frame(pointer(import, false));
    let dialog = frame(vec![]);
    assert!(label_center(&dialog, "Import Player Markings").is_some());
    assert!(label_center(&dialog, "Replace All Existing Markings and Notes").is_some());
    assert!(label_center(&dialog, "Import").is_some());
    assert!(label_center(&dialog, "Replace All").is_none());
}
