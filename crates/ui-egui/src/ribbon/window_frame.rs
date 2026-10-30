use super::{SheetApp, Tokens, json};

/// Width of one window-control button on the title bar.
pub const CONTROL_W: f32 = 46.0;
const TITLE_H: f32 = 38.0;
const RESIZE_BAND: f32 = 5.0;

/// Draw application controls and edge resizing for frameless windows.
pub fn window_frame(app: &mut SheetApp, ctx: &egui::Context) {
    if !app.window_controls {
        return;
    }
    let (maximized, fullscreen) = ctx.input(|i| (i.viewport().maximized.unwrap_or(false), i.viewport().fullscreen.unwrap_or(false)));
    if fullscreen {
        return;
    }
    let t = Tokens::get(ctx);
    let screen = ctx.content_rect();
    let controls = egui::Rect::from_min_size(egui::pos2(screen.max.x - 3.0 * CONTROL_W, screen.min.y), egui::vec2(3.0 * CONTROL_W, TITLE_H));
    egui::Area::new(egui::Id::unique("gc_window_controls")).order(egui::Order::Foreground).fixed_pos(controls.min).show(ctx, |ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        ui.horizontal(|ui| {
            for kind in ["minimize", "maximize", "close"] {
                let (r, resp) = ui.allocate_exact_size(egui::vec2(CONTROL_W, TITLE_H), egui::Sense::click());
                let label = app.ui.language.tr(match kind {
                    "minimize" => "Minimize",
                    "maximize" if maximized => "Restore Down",
                    "maximize" => "Maximize",
                    _ => "Close",
                });
                let hot = resp.hovered() || resp.is_pointer_button_down_on();
                if hot {
                    ui.painter().rect_filled(
                        r,
                        0.0,
                        if kind == "close" {
                            egui::Color32::from_rgb(0xC4, 0x2B, 0x1C)
                        } else if resp.is_pointer_button_down_on() {
                            t.pressed
                        } else {
                            t.hover
                        },
                    );
                }
                resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label));
                let m = r.center();
                let stroke = egui::Stroke::new(1.0, if kind == "close" && hot { egui::Color32::WHITE } else { t.text });
                match kind {
                    "minimize" => {
                        ui.painter().line_segment([egui::pos2(m.x - 5.0, m.y), egui::pos2(m.x + 5.0, m.y)], stroke);
                    }
                    "maximize" if maximized => {
                        let front = egui::Rect::from_min_size(egui::pos2(m.x - 5.0, m.y - 3.0), egui::vec2(8.0, 8.0));
                        ui.painter().rect_stroke(front, 0.0, stroke, egui::StrokeKind::Inside);
                        ui.painter().line_segment([egui::pos2(m.x - 3.0, m.y - 5.0), egui::pos2(m.x + 5.0, m.y - 5.0)], stroke);
                        ui.painter().line_segment([egui::pos2(m.x + 5.0, m.y - 5.0), egui::pos2(m.x + 5.0, m.y + 3.0)], stroke);
                    }
                    "maximize" => {
                        ui.painter().rect_stroke(r.shrink(15.0), 0.0, stroke, egui::StrokeKind::Inside);
                    }
                    _ => {
                        ui.painter().line_segment([m - egui::vec2(5.0, 5.0), m + egui::vec2(5.0, 5.0)], stroke);
                        ui.painter().line_segment([m + egui::vec2(5.0, -5.0), m - egui::vec2(5.0, -5.0)], stroke);
                    }
                }
                if resp.on_hover_text(label).clicked() {
                    match kind {
                        "minimize" => app.run_or_alert("ui.window.minimize", json!({})),
                        "maximize" => app.run_or_alert("ui.window.maximize", json!({"on": !maximized})),
                        "close" => app.run_or_alert("ui.window.close", json!({})),
                        _ => {}
                    }
                }
            }
        });
    });
    if maximized {
        return;
    }
    let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) else {
        return;
    };
    let l = pos.x <= screen.min.x + RESIZE_BAND;
    let r = pos.x >= screen.max.x - RESIZE_BAND;
    let top = pos.y <= screen.min.y + RESIZE_BAND;
    let b = pos.y >= screen.max.y - RESIZE_BAND;
    let dir = match (l, r, top, b) {
        (true, false, true, false) => Some(egui::viewport::ResizeDirection::NorthWest),
        (false, true, true, false) => Some(egui::viewport::ResizeDirection::NorthEast),
        (true, false, false, true) => Some(egui::viewport::ResizeDirection::SouthWest),
        (false, true, false, true) => Some(egui::viewport::ResizeDirection::SouthEast),
        (true, false, _, _) => Some(egui::viewport::ResizeDirection::West),
        (false, true, _, _) => Some(egui::viewport::ResizeDirection::East),
        (_, _, true, false) => Some(egui::viewport::ResizeDirection::North),
        (_, _, false, true) => Some(egui::viewport::ResizeDirection::South),
        _ => None,
    };
    if let Some(dir) = dir
        && ctx.input(|i| i.pointer.primary_pressed())
        && !controls.contains(pos)
    {
        let direction = match dir {
            egui::ResizeDirection::North => "north",
            egui::ResizeDirection::South => "south",
            egui::ResizeDirection::East => "east",
            egui::ResizeDirection::West => "west",
            egui::ResizeDirection::NorthEast => "northeast",
            egui::ResizeDirection::NorthWest => "northwest",
            egui::ResizeDirection::SouthEast => "southeast",
            egui::ResizeDirection::SouthWest => "southwest",
        };
        app.run_or_alert("ui.window.beginResize", json!({"direction": direction}));
    }
    app.flush_viewport_commands(ctx);
}
