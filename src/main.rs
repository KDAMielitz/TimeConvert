#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
mod logic;
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use chrono_tz::Tz;
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, str::FromStr};

#[derive(Serialize, Deserialize)]
#[serde(default)]
struct Settings {
    zones: Vec<String>,
    on_top: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            zones: vec!["America/Chicago".into(), "Asia/Tokyo".into()],
            on_top: false,
        }
    }
}
impl Settings {
    fn normalize(&mut self) {
        self.zones.truncate(6);
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
#[derive(Default)]
struct Row {
    query: String,
    popup_open: bool,
    date: String,
    time: String,
    message: String,
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
        let rows = (0..settings.zones.len()).map(|_| Row::default()).collect();
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
        Tz::from_str(&self.settings.zones[i]).unwrap_or(chrono_tz::UTC)
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
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.strong(((b'A' + i as u8) as char).to_string());
                            let selected = self.settings.zones[i].clone();
                            let popup = egui::ComboBox::from_id_salt(i)
                                .selected_text(&selected)
                                .width(255.0)
                                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                                .show_ui(ui, |ui| {
                                    let search = ui.add(
                                        egui::TextEdit::singleline(&mut self.rows[i].query)
                                            .hint_text("Search country, city, or IANA zone"),
                                    );
                                    if !self.rows[i].popup_open {
                                        search.request_focus();
                                    }
                                    self.rows[i].popup_open = true;
                                    let query = self.rows[i].query.to_lowercase();
                                    egui::ScrollArea::vertical()
                                        .max_height(200.0)
                                        .show(ui, |ui| {
                                            for z in self
                                                .zones
                                                .iter()
                                                .filter(|z| logic::matches(z, &query))
                                            {
                                                let offset =
                                                    self.instant.with_timezone(&z.tz).format("%:z");
                                                if ui
                                                    .selectable_label(
                                                        selected == z.tz.name(),
                                                        format!("{} (UTC{})", z.label, offset),
                                                    )
                                                    .clicked()
                                                {
                                                    self.settings.zones[i] = z.tz.name().into();
                                                    self.rows[i].query.clear();
                                                    self.rows[i].popup_open = false;
                                                    ui.memory_mut(|m| m.close_popup());
                                                    break;
                                                }
                                            }
                                        });
                                });
                            if popup.inner.is_none() {
                                self.rows[i].popup_open = false;
                            }
                            if selected != self.settings.zones[i] {
                                self.refresh_row(i);
                                save(&self.settings);
                            }
                            if i >= 2 && ui.small_button("×").on_hover_text("Remove zone").clicked()
                            {
                                remove = Some(i);
                            }
                        });
                        ui.horizontal(|ui| {
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
                            if i == 0 {
                                let enter_key = (date.has_focus()
                                    || date.lost_focus()
                                    || time.has_focus()
                                    || time.lost_focus())
                                    && ui.input(|input| input.key_pressed(egui::Key::Enter));
                                if ui.button("Enter").clicked() || enter_key {
                                    self.convert(i);
                                }
                                if ui.small_button("Now").clicked() {
                                    self.instant = Utc::now();
                                    self.reference = 0;
                                    self.refresh();
                                }
                            }
                        });
                        let t = self.instant.with_timezone(&self.tz(i));
                        let base = self.instant.with_timezone(&self.tz(self.reference));
                        let diff = (t.date_naive() - base.date_naive()).num_days();
                        ui.label(format!(
                            "{} UTC{}{}",
                            t.format("%Z"),
                            t.format("%:z"),
                            logic::day_label(diff, self.reference, i)
                        ));
                        if !self.rows[i].message.is_empty() {
                            ui.label(&self.rows[i].message);
                        }
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
                        .add_enabled(self.rows.len() < 6, egui::Button::new("+ Add zone"))
                        .clicked()
                    {
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
    eframe::run_native(
        "Time Convert",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([420.0, 290.0])
                .with_min_inner_size([350.0, 240.0]),
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
        };
        s.normalize();
        assert_eq!(s.zones.len(), 2);
        s.zones = vec!["UTC".into(); 9];
        s.normalize();
        assert_eq!(s.zones.len(), 6);
    }
}
