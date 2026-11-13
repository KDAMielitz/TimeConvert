#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
mod logic;
use chrono::{DateTime, NaiveDate, NaiveDateTime, Timelike, Utc};
use chrono_tz::Tz;
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, str::FromStr};

const BACKGROUND: egui::Color32 = egui::Color32::from_rgb(13, 24, 36);
const CARD: egui::Color32 = egui::Color32::from_rgb(22, 38, 53);
const ACCENT: egui::Color32 = egui::Color32::from_rgb(58, 210, 194);
const MUTED: egui::Color32 = egui::Color32::from_rgb(159, 181, 194);

fn configure_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 7.0);
    style.spacing.button_padding = egui::vec2(9.0, 5.0);
    style.visuals = egui::Visuals::dark();
    style.visuals.panel_fill = BACKGROUND;
    style.visuals.window_fill = CARD;
    style.visuals.extreme_bg_color = egui::Color32::from_rgb(12, 27, 39);
    style.visuals.widgets.noninteractive.bg_fill = CARD;
    style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(37, 57, 72);
    style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(48, 78, 90);
    style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(37, 115, 116);
    style.visuals.selection.bg_fill = egui::Color32::from_rgb(27, 111, 111);
    style.visuals.window_corner_radius = egui::CornerRadius::same(10);
    for widget in [
        &mut style.visuals.widgets.noninteractive,
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
        &mut style.visuals.widgets.open,
    ] {
        widget.corner_radius = egui::CornerRadius::same(6);
    }
    ctx.set_style(style);
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
struct Settings {
    zones: Vec<String>,
    on_top: bool,
    analog: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            zones: vec!["America/Chicago".into(), "Asia/Tokyo".into()],
            on_top: false,
            analog: false,
        }
    }
}
impl Settings {
    fn normalize(&mut self) {
        while self.zones.len() < 2 {
            self.zones.push("Asia/Tokyo".into());
        }
    }
}
fn settings_path() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|p| PathBuf::from(p).join("TimeConvert").join("settings.json"))
}
fn save(s: &Settings) {
    if let Some(path) = settings_path() {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(data) = serde_json::to_vec(s) {
            let _ = std::fs::write(path, data);
        }
    }
}
struct Row {
    tz: Tz,
    query: String,
    popup_open: bool,
    date: String,
    time: String,
    message: String,
}
impl Default for Row {
    fn default() -> Self {
        Self {
            tz: chrono_tz::UTC,
            query: String::new(),
            popup_open: false,
            date: String::new(),
            time: String::new(),
            message: String::new(),
        }
    }
}
struct App {
    settings: Settings,
    zones: Vec<logic::Zone>,
    rows: Vec<Row>,
    instant: DateTime<Utc>,
    reference: usize,
}
impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        configure_style(&cc.egui_ctx);
        let mut settings: Settings = settings_path()
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        settings.normalize();
        cc.egui_ctx
            .send_viewport_cmd(egui::ViewportCommand::WindowLevel(if settings.on_top {
                egui::WindowLevel::AlwaysOnTop
            } else {
                egui::WindowLevel::Normal
            }));
        let rows = settings
            .zones
            .iter()
            .map(|zone| Row {
                tz: Tz::from_str(zone).unwrap_or(chrono_tz::UTC),
                ..Default::default()
            })
            .collect();
        let mut a = Self {
            settings,
            zones: logic::zones(),
            rows,
            instant: Utc::now(),
            reference: 0,
        };
        a.refresh();
        a
    }
    fn tz(&self, i: usize) -> Tz {
        self.rows[i].tz
    }
    fn refresh_row(&mut self, i: usize) {
        let t = self.instant.with_timezone(&self.tz(i));
        self.rows[i].date = t.format("%Y-%m-%d").to_string();
        self.rows[i].time = t.format("%H:%M").to_string();
        self.rows[i].message.clear();
    }
    fn refresh(&mut self) {
        for i in 0..self.rows.len() {
            self.refresh_row(i);
        }
    }
    fn convert(&mut self, i: usize) {
        let parsed = NaiveDate::parse_from_str(&self.rows[i].date, "%Y-%m-%d")
            .ok()
            .zip(logic::parse_time(&self.rows[i].time));
        if let Some((date, time)) = parsed {
            if let Some(res) = logic::resolve(self.tz(i), NaiveDateTime::new(date, time)) {
                self.instant = res.instant;
                self.reference = i;
                self.refresh();
                self.rows[i].message = res.note.unwrap_or_default();
                return;
            }
        }
        self.rows[i].message = "Enter a valid date and time".into();
    }
}
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Time Convert");
                if ui
                    .checkbox(&mut self.settings.on_top, "Always on top")
                    .changed()
                {
                    ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
                        if self.settings.on_top {
                            egui::WindowLevel::AlwaysOnTop
                        } else {
                            egui::WindowLevel::Normal
                        },
                    ));
                    save(&self.settings);
                }
                if ui.button("Swap A/B").clicked() {
                    self.settings.zones.swap(0, 1);
                    self.rows.swap(0, 1);
                    if self.reference < 2 {
                        self.reference = 1 - self.reference;
                    }
                    save(&self.settings);
                }
            });
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let mut remove = None;
                    for i in 0..self.rows.len() {
                        egui::Frame::new()
                            .fill(CARD)
                            .stroke(egui::Stroke::new(
                                1.0_f32,
                                if i == 0 {
                                    ACCENT
                                } else {
                                    egui::Color32::from_rgb(43, 65, 78)
                                },
                            ))
                            .corner_radius(egui::CornerRadius::same(10))
                            .inner_margin(10)
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    let (badge, _) = ui.allocate_exact_size(
                                        egui::vec2(26.0, 26.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().circle_filled(
                                        badge.center(),
                                        13.0,
                                        if i == 0 {
                                            egui::Color32::from_rgb(25, 110, 111)
                                        } else {
                                            egui::Color32::from_rgb(42, 76, 89)
                                        },
                                    );
                                    ui.painter().text(
                                        badge.center(),
                                        egui::Align2::CENTER_CENTER,
                                        logic::row_label(i),
                                        egui::FontId::proportional(12.0),
                                        egui::Color32::WHITE,
                                    );
                                    let selected = self.settings.zones[i].clone();
                                    let popup = egui::ComboBox::from_id_salt(i)
                                        .selected_text(&selected)
                                        .width(
                                            (ui.available_width()
                                                - if i >= 2 {
                                                    80.0
                                                } else if i == 0 {
                                                    62.0
                                                } else {
                                                    8.0
                                                })
                                            .clamp(120.0, 310.0),
                                        )
                                        .close_behavior(
                                            egui::PopupCloseBehavior::CloseOnClickOutside,
                                        )
                                        .show_ui(ui, |ui| {
                                            let search = ui.add(
                                                egui::TextEdit::singleline(&mut self.rows[i].query)
                                                    .hint_text(
                                                        "Search country, city, or IANA zone",
                                                    ),
                                            );
                                            if !self.rows[i].popup_open {
                                                search.request_focus();
                                            }
                                            self.rows[i].popup_open = true;
                                            let query = self.rows[i].query.to_lowercase();
                                            egui::ScrollArea::vertical().max_height(200.0).show(
                                                ui,
                                                |ui| {
                                                    for z in self
                                                        .zones
                                                        .iter()
                                                        .filter(|z| logic::matches(z, &query))
                                                    {
                                                        let offset = self
                                                            .instant
                                                            .with_timezone(&z.tz)
                                                            .format("%:z");
                                                        if ui
                                                            .selectable_label(
                                                                selected == z.tz.name(),
                                                                format!(
                                                                    "{} (UTC{})",
                                                                    z.label, offset
                                                                ),
                                                            )
                                                            .clicked()
                                                        {
                                                            self.settings.zones[i] =
                                                                z.tz.name().into();
                                                            self.rows[i].tz = z.tz;
                                                            self.rows[i].query.clear();
                                                            self.rows[i].popup_open = false;
                                                            ui.memory_mut(|m| m.close_popup());
                                                            break;
                                                        }
                                                    }
                                                },
                                            );
                                        });
                                    if popup.inner.is_none() {
                                        self.rows[i].popup_open = false;
                                    }
                                    if selected != self.settings.zones[i] {
                                        self.refresh_row(i);
                                        save(&self.settings);
                                    }
                                    if i == 0 && ui.small_button("Now").clicked() {
                                        self.instant = Utc::now();
                                        self.reference = 0;
                                        self.refresh();
                                    }
                                    if i >= 2
                                        && ui
                                            .small_button("Remove")
                                            .on_hover_text("Remove zone")
                                            .clicked()
                                    {
                                        remove = Some(i);
                                    }
                                });
                                ui.horizontal(|ui| {
                                    if i == 0 {
                                        let date = ui.add_sized(
                                            [95.0, 20.0],
                                            egui::TextEdit::singleline(&mut self.rows[i].date)
                                                .hint_text("YYYY-MM-DD")
                                                .interactive(i == 0),
                                        );
                                        let time = ui.add_sized(
                                            [55.0, 20.0],
                                            egui::TextEdit::singleline(&mut self.rows[i].time)
                                                .hint_text("HH:MM")
                                                .interactive(i == 0),
                                        );
                                        if date.changed() || time.changed() {
                                            self.rows[i].message.clear();
                                        }
                                        let enter_key = (date.has_focus()
                                            || date.lost_focus()
                                            || time.has_focus()
                                            || time.lost_focus())
                                            && ui
                                                .input(|input| input.key_pressed(egui::Key::Enter));
                                        if ui
                                            .add(
                                                egui::Button::new(
                                                    egui::RichText::new("Enter")
                                                        .color(BACKGROUND)
                                                        .strong(),
                                                )
                                                .fill(ACCENT),
                                            )
                                            .clicked()
                                            || enter_key
                                        {
                                            self.convert(i);
                                        }
                                    } else {
                                        ui.label(
                                            egui::RichText::new(&self.rows[i].time)
                                                .monospace()
                                                .size(27.0)
                                                .strong()
                                                .color(egui::Color32::WHITE),
                                        );
                                    }
                                });
                                let t = self.instant.with_timezone(&self.tz(i));
                                if self.settings.analog {
                                    ui.horizontal(|ui| {
                                        let (rect, _) = ui.allocate_exact_size(
                                            egui::vec2(42.0, 42.0),
                                            egui::Sense::hover(),
                                        );
                                        let center = rect.center();
                                        let painter = ui.painter();
                                        painter.circle_filled(center, 20.0, BACKGROUND);
                                        painter.circle_stroke(
                                            center,
                                            19.0,
                                            egui::Stroke::new(1.5_f32, ACCENT),
                                        );
                                        let minute = t.minute() as f32;
                                        let hour = (t.hour() % 12) as f32 + minute / 60.0;
                                        for (fraction, length, width) in [
                                            (hour / 12.0, 10.0, 2.5_f32),
                                            (minute / 60.0, 15.0, 1.8_f32),
                                        ] {
                                            let angle = fraction * std::f32::consts::TAU;
                                            let tip = center
                                                + egui::vec2(angle.sin(), -angle.cos()) * length;
                                            painter.line_segment(
                                                [center, tip],
                                                egui::Stroke::new(width, egui::Color32::WHITE),
                                            );
                                        }
                                        painter.circle_filled(center, 2.0, ACCENT);
                                        ui.label(
                                            egui::RichText::new("Local time").small().color(MUTED),
                                        );
                                    });
                                }
                                let base = self.instant.with_timezone(&self.tz(self.reference));
                                let diff = (t.date_naive() - base.date_naive()).num_days();
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{}  ·  {} UTC{}",
                                            self.rows[i].date,
                                            t.format("%Z"),
                                            t.format("%:z"),
                                        ))
                                        .small()
                                        .color(MUTED),
                                    );
                                    egui::Frame::new()
                                        .fill(egui::Color32::from_rgb(31, 65, 72))
                                        .corner_radius(egui::CornerRadius::same(6))
                                        .inner_margin(egui::Margin::symmetric(6, 2))
                                        .show(ui, |ui| {
                                            ui.label(
                                                egui::RichText::new(
                                                    logic::day_label(diff, self.reference, i)
                                                        .trim(),
                                                )
                                                .small()
                                                .color(ACCENT),
                                            );
                                        });
                                });
                                if !self.rows[i].message.is_empty() {
                                    ui.label(&self.rows[i].message);
                                }
                            });
                        ui.add_space(4.0);
                    }
                    if let Some(i) = remove {
                        self.rows.remove(i);
                        self.settings.zones.remove(i);
                        if self.reference == i {
                            self.reference = 0;
                        } else if self.reference > i {
                            self.reference -= 1;
                        }
                        save(&self.settings);
                    }
                    ui.separator();
                    if ui
                        .checkbox(&mut self.settings.analog, "Analog clocks")
                        .changed()
                    {
                        save(&self.settings);
                    }
                    if ui.button("+ Add zone").clicked() {
                        self.settings.zones.push("Etc/UTC".into());
                        self.rows.push(Row::default());
                        self.refresh_row(self.rows.len() - 1);
                        save(&self.settings);
                    }
                });
        });
    }
}
fn main() -> eframe::Result {
    let icon = image::load_from_memory(include_bytes!("../assets/icon.png"))
        .expect("embedded icon")
        .into_rgba8();
    let (width, height) = icon.dimensions();
    eframe::run_native(
        "Time Convert",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([420.0, 290.0])
                .with_min_inner_size([350.0, 240.0])
                .with_icon(egui::IconData {
                    rgba: icon.into_raw(),
                    width,
                    height,
                }),
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn settings_defaults_missing_fields() {
        let s: Settings = serde_json::from_str(r#"{"on_top":true}"#).unwrap();
        assert!(s.on_top);
        assert_eq!(s.zones, Settings::default().zones);
    }
    #[test]
    fn old_two_zone_settings_load() {
        let s: Settings =
            serde_json::from_str(r#"{"zones":["Europe/London","Asia/Tokyo"],"on_top":true}"#)
                .unwrap();
        assert_eq!(s.zones, vec!["Europe/London", "Asia/Tokyo"]);
        assert!(s.on_top);
    }
    #[test]
    fn settings_row_bounds() {
        let mut s = Settings {
            zones: vec![],
            on_top: false,
            analog: false,
        };
        s.normalize();
        assert_eq!(s.zones.len(), 2);
        s.zones = vec!["UTC".into(); 9];
        s.normalize();
        assert_eq!(s.zones.len(), 9);
    }
}
