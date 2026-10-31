use super::show::{ok_cancel_enabled, track_pick};
use super::{CellColor, CellRef, CfRule, CfValueKind, Color32, Dialog, Json, RefPick, Sense, Stroke, StrokeKind, json, vec2};

fn cf_stop_json(kind: CfValueKind, color: &CellColor, theme: &gridcraft_engine::model::Theme) -> Json {
    let (ty, value) = match kind {
        CfValueKind::Min => ("min", Json::Null),
        CfValueKind::Max => ("max", Json::Null),
        CfValueKind::Number(n) => ("num", json!(n)),
        CfValueKind::Percent(n) => ("percent", json!(n)),
        CfValueKind::Percentile(n) => ("percentile", json!(n)),
        CfValueKind::Formula(f) => ("formula", json!(f)),
    };
    let hex = color.hex(theme).unwrap_or_else(|| "#000000".into());
    // `source` keeps a theme colour by reference: OK without touching the picker sends it back
    // unchanged, instead of freezing the theme's current RGB into the rule.
    json!({"type": ty, "value": value, "color": hex.clone(), "source": color, "sourceHex": hex})
}

/// The stop as the engine takes it, dropping the dialog's bookkeeping. A colour the user did not
/// change keeps its original theme reference; a changed one is sent as the picked RGB.
pub(super) fn cf_stop_params(stop: &Json) -> Json {
    let mut obj = stop.as_object().cloned().unwrap_or_default();
    let source = obj.remove("source").and_then(|v| serde_json::from_value::<CellColor>(v).ok());
    let source_hex = obj.remove("sourceHex").and_then(|v| v.as_str().map(str::to_string));
    if let (Some(source), Some(source_hex)) = (source, source_hex)
        && obj.get("color").and_then(Json::as_str).is_some_and(|c| c.eq_ignore_ascii_case(&source_hex))
    {
        obj.insert(
            "color".into(),
            match source {
                CellColor::Auto => json!("auto"),
                CellColor::Rgb(v) => json!(format!("#{v:06X}")),
                CellColor::Theme(i, tint) => json!({"theme": i, "tint": tint as f64 / 1000.0}),
            },
        );
    }
    Json::Object(obj)
}

fn default_scale_stops(style: &str) -> Vec<Json> {
    if style == "2color" {
        vec![json!({"type": "min", "value": null, "color": "#F8696B"}), json!({"type": "max", "value": null, "color": "#63BE7B"})]
    } else {
        vec![
            json!({"type": "min", "value": null, "color": "#F8696B"}),
            json!({"type": "percentile", "value": 50, "color": "#FFEB84"}),
            json!({"type": "max", "value": null, "color": "#63BE7B"}),
        ]
    }
}

pub(super) fn new_rule_defaults(params: &Json, theme: &gridcraft_engine::model::Theme) -> Json {
    // Excel's New Formatting Rule opens on the first type. Manage Rules edits a colour scale,
    // so an edit always stays on that branch.
    let mut rule_type = params.get("ruleType").and_then(Json::as_str).unwrap_or("allCells").to_string();
    if params.get("edit").is_some() || !matches!(rule_type.as_str(), "allCells" | "formula") {
        rule_type = "allCells".into();
    }
    let mut style = params.get("style").and_then(Json::as_str).unwrap_or("3color").to_string();
    if !matches!(style.as_str(), "2color" | "3color") {
        style = "3color".into();
    }
    let existing = params.get("rule").and_then(|rule| serde_json::from_value::<CfRule>(rule.clone()).ok()).and_then(|rule| match rule {
        CfRule::ColorScale { stops } if (2..=3).contains(&stops.len()) => {
            Some(stops.into_iter().map(|(kind, color)| cf_stop_json(kind, &color, theme)).collect::<Vec<_>>())
        }
        _ => None,
    });
    let stops = match existing {
        Some(stops) => {
            style = if stops.len() == 2 { "2color".into() } else { "3color".into() };
            stops
        }
        None => default_scale_stops(&style),
    };
    json!({
        "ruleType": rule_type,
        "style": style,
        "stops": stops,
        "formula": "",
        "preset": "lightRedFillDarkRedText",
        "edit": params.get("edit").cloned(),
    })
}

fn parse_hex_color(s: &str) -> Color32 {
    let h = s.trim_start_matches('#');
    u32::from_str_radix(h.get(..6).unwrap_or("000000"), 16)
        .map(|v| Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8))
        .unwrap_or(Color32::WHITE)
}

fn hex_color(c: Color32) -> String {
    format!("#{:02X}{:02X}{:02X}", c.r(), c.g(), c.b())
}

fn scale_preview(ui: &mut egui::Ui, stops: &[Json]) {
    let (rect, _) = ui.allocate_exact_size(vec2(280.0, 26.0), Sense::hover());
    let colors: Vec<Color32> = stops.iter().map(|s| parse_hex_color(s.get("color").and_then(Json::as_str).unwrap_or("#FFFFFF"))).collect();
    if colors.is_empty() {
        return;
    }
    let n = 64;
    for i in 0..n {
        let t = i as f32 / (n - 1) as f32;
        let pos = t * (colors.len() - 1) as f32;
        let lo = pos.floor() as usize;
        let hi = (lo + 1).min(colors.len() - 1);
        let f = pos - lo as f32;
        let a = colors[lo];
        let b = colors[hi];
        let c = Color32::from_rgb(
            (a.r() as f32 + (b.r() as f32 - a.r() as f32) * f) as u8,
            (a.g() as f32 + (b.g() as f32 - a.g() as f32) * f) as u8,
            (a.b() as f32 + (b.b() as f32 - a.b() as f32) * f) as u8,
        );
        let x = rect.left() + rect.width() * i as f32 / n as f32;
        ui.painter().rect_filled(egui::Rect::from_min_size(egui::pos2(x, rect.top()), vec2(rect.width() / n as f32 + 0.5, rect.height())), 0.0, c);
    }
    ui.painter().rect_stroke(rect, 0.0, Stroke::new(1.0, Color32::from_gray(190)), StrokeKind::Inside);
}

fn stop_combo(ui: &mut egui::Ui, id: (&'static str, usize), current: &mut String, options: &[(&str, &str)], lang: crate::i18n::Language) {
    let shown = options.iter().find(|(v, _)| *v == current).map(|(_, l)| lang.tr(l)).unwrap_or(current.as_str());
    craft_ui::ComboBox::from_id_salt(id).selected_text(shown).width(120.0).show_ui(ui, |ui| {
        for (v, l) in options {
            if ui.selectable_label(current == v, lang.tr(l)).clicked() {
                *current = (*v).to_string();
            }
        }
    });
}

fn colour_scale_editor(ui: &mut egui::Ui, d: &mut Dialog, lang: crate::i18n::Language) {
    ui.label(lang.tr("Format all cells based on their values"));
    ui.horizontal(|ui| {
        ui.label(lang.tr("Format Style:"));
        let mut style = d.values.get("style").and_then(Json::as_str).unwrap_or("3color").to_string();
        craft_ui::ComboBox::from_id_salt("newFormattingRule.style")
            .selected_text(lang.tr(if style == "2color" { "2-Color Scale" } else { "3-Color Scale" }))
            .width(150.0)
            .show_ui(ui, |ui| {
                for (v, l) in [("2color", "2-Color Scale"), ("3color", "3-Color Scale")] {
                    if ui.selectable_label(style == v, lang.tr(l)).clicked() {
                        style = v.to_string();
                    }
                }
            });
        d.values.insert("style".into(), json!(style));
        let mut stops = d.values.get("stops").and_then(Json::as_array).cloned().unwrap_or_default();
        if style == "2color" && stops.len() == 3 {
            stops.remove(1);
        } else if style == "3color" && stops.len() == 2 {
            stops.insert(1, json!({"type": "percentile", "value": 50, "color": "#FFEB84"}));
        }
        d.values.insert("stops".into(), Json::Array(stops));
    });
    let mut stops = d.values.get("stops").and_then(Json::as_array).cloned().unwrap_or_default();
    let count = stops.len();
    for i in 0..count {
        let mut obj = stops.get(i).and_then(Json::as_object).cloned().unwrap_or_default();
        let mut ty = obj.get("type").and_then(Json::as_str).unwrap_or("num").to_string();
        let options: &[(&str, &str)] = if i == 0 {
            &[("min", "Lowest value"), ("num", "Number"), ("percent", "Percent"), ("percentile", "Percentile"), ("formula", "Formula")]
        } else if i + 1 == count {
            &[("max", "Highest value"), ("num", "Number"), ("percent", "Percent"), ("percentile", "Percentile"), ("formula", "Formula")]
        } else {
            &[("num", "Number"), ("percent", "Percent"), ("percentile", "Percentile"), ("formula", "Formula")]
        };
        ui.horizontal(|ui| {
            ui.label(lang.tr(if i == 0 {
                "Minimum"
            } else if i + 1 == count {
                "Maximum"
            } else {
                "Midpoint"
            }));
            stop_combo(ui, ("newFormattingRule.stopType", i), &mut ty, options, lang);
            match ty.as_str() {
                "min" | "max" => {
                    ui.label(lang.tr(if ty == "min" { "Lowest value" } else { "Highest value" }));
                }
                "formula" => {
                    let mut f = obj.get("value").and_then(Json::as_str).unwrap_or("").to_string();
                    ui.add(egui::TextEdit::singleline(&mut f).desired_width(120.0).hint_text(lang.tr("Formula")));
                    // Store what the field shows as a string even when it was not edited: a
                    // stop switched from Number still holds that number, which is no formula.
                    obj.insert("value".into(), json!(f));
                }
                _ => {
                    let mut n = obj.get("value").and_then(Json::as_f64).unwrap_or(0.0);
                    let speed = if ty == "num" { 1.0 } else { 0.1 };
                    if ui.add(egui::DragValue::new(&mut n).speed(speed).range(if ty == "num" { f64::MIN..=f64::MAX } else { 0.0..=100.0 })).changed()
                    {
                        obj.insert("value".into(), json!(n));
                    }
                    // A numeric stop must carry a number: a stop switched from Lowest or
                    // Highest value still holds `null`, which the engine refuses.
                    if obj.get("value").and_then(Json::as_f64).is_none() {
                        obj.insert("value".into(), json!(n));
                    }
                }
            }
            let c = parse_hex_color(obj.get("color").and_then(Json::as_str).unwrap_or("#FFFFFF"));
            let mut rgb = [c.r(), c.g(), c.b()];
            if ui.color_edit_button_srgb(&mut rgb).changed() {
                obj.insert("color".into(), json!(hex_color(Color32::from_rgb(rgb[0], rgb[1], rgb[2]))));
            }
        });
        obj.insert("type".into(), json!(ty));
        stops[i] = Json::Object(obj);
    }
    d.values.insert("stops".into(), Json::Array(stops.clone()));
    scale_preview(ui, &stops);
}

fn preset_combo(ui: &mut egui::Ui, d: &mut Dialog, lang: crate::i18n::Language) {
    const PRESETS: &[(&str, &str)] = &[
        ("lightRedFillDarkRedText", "Light Red Fill with Dark Red Text"),
        ("yellowFillDarkYellowText", "Yellow Fill with Dark Yellow Text"),
        ("greenFillDarkGreenText", "Green Fill with Dark Green Text"),
        ("lightRedFill", "Light Red Fill"),
        ("redText", "Red Text"),
        ("redBorder", "Red Border"),
    ];
    let mut preset = d.values.get("preset").and_then(Json::as_str).unwrap_or("lightRedFillDarkRedText").to_string();
    let shown = PRESETS.iter().find(|(v, _)| *v == preset).map(|(_, l)| lang.tr(l)).unwrap_or(preset.as_str());
    craft_ui::ComboBox::from_id_salt("newFormattingRule.preset").selected_text(shown).width(260.0).show_ui(ui, |ui| {
        for (v, l) in PRESETS {
            if ui.selectable_label(preset == *v, lang.tr(l)).clicked() {
                preset = (*v).to_string();
            }
        }
    });
    d.values.insert("preset".into(), json!(preset));
}

/// Excel's six rule types, in its order.
const RULE_TYPES: &[(&str, &str)] = &[
    ("allCells", "Format all cells based on their values"),
    ("cellsThatContain", "Format only cells that contain"),
    ("topBottom", "Format only top or bottom ranked values"),
    ("aboveBelow", "Format only values that are above or below average"),
    ("uniqueDuplicate", "Format only unique or duplicate values"),
    ("formula", "Use a formula to determine which cells to format"),
];

/// Gives a formula rule's box point mode, on the sheet and selection the dialog opened on;
/// other rule types take clicks on the grid as selection.
pub(super) fn sync_rule_pick(d: &mut Dialog) {
    if d.values.get("ruleType").and_then(Json::as_str) != Some("formula") {
        d.pick = None;
        return;
    }
    if d.pick.is_some() {
        return;
    }
    let sheet = d.values.get("sheet").and_then(Json::as_str).unwrap_or_default().to_string();
    let applies_to = d.values.get("applies_to").and_then(Json::as_str).unwrap_or_default().to_string();
    d.pick = Some(RefPick { key: "formula", sheet, caret: 0, point: None, anchor: CellRef::new(0, 0), refocus: false, applies_to });
}

pub(super) fn new_formatting_rule(ui: &mut egui::Ui, d: &mut Dialog, confirm: &mut bool, open: &mut bool) {
    let lang = crate::i18n::current(ui.ctx());
    ui.label(lang.tr("Select a Rule Type"));
    let mut rule_type = d.values.get("ruleType").and_then(Json::as_str).unwrap_or("allCells").to_string();
    for (v, label) in RULE_TYPES {
        ui.selectable_value(&mut rule_type, (*v).to_string(), lang.tr(label));
    }
    d.values.insert("ruleType".into(), json!(rule_type));
    // Only a formula rule's box takes grid references, and it keeps the sheet and "Applies to"
    // range the dialog opened on.
    sync_rule_pick(d);
    ui.separator();
    ui.label(lang.tr("Edit the Rule Description"));
    let supported = match rule_type.as_str() {
        "allCells" => {
            colour_scale_editor(ui, d, lang);
            // A formula stop needs a formula: one whose value is missing, not a string, or
            // blank would create a rule that never applies. Other stops confirm as they are.
            d.values.get("stops").and_then(Json::as_array).is_none_or(|stops| {
                stops.iter().all(|s| {
                    s.get("type").and_then(Json::as_str) != Some("formula")
                        || s.get("value").and_then(Json::as_str).is_some_and(|f| !f.trim().is_empty())
                })
            })
        }
        "formula" => {
            ui.label(lang.tr("Format values where this formula is true:"));
            let mut formula = d.values.get("formula").and_then(Json::as_str).unwrap_or("").to_string();
            let r = ui.add(egui::TextEdit::singleline(&mut formula).desired_width(360.0));
            if r.changed() {
                d.values.insert("formula".into(), json!(formula.clone()));
            }
            if let Some(pick) = d.pick.as_mut().filter(|pick| pick.key == "formula") {
                track_pick(pick, &r, r.changed());
            }
            ui.horizontal(|ui| {
                ui.label(lang.tr("with"));
                preset_combo(ui, d, lang);
            });
            // A formula rule with no formula never applies; OK waits for one.
            !formula.trim().is_empty()
        }
        _ => {
            ui.label(lang.tr("This rule type isn't available yet."));
            false
        }
    };
    ok_cancel_enabled(ui, confirm, open, supported);
}
