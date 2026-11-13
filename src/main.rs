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
    time_12h: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            zones: vec!["America/Chicago".into(), "Asia/Tokyo".into()],
            on_top: false,
            analog: false,
            time_12h: false,
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
fn settings_path_for(
    os: &str,
    appdata: Option<PathBuf>,
    xdg: Option<PathBuf>,
    home: Option<PathBuf>,
) -> Option<PathBuf> {
    let dir = match os {
        "windows" => appdata?.join("TimeConvert"),
        "macos" => home?
            .join("Library")
            .join("Application Support")
            .join("TimeConvert"),
        _ => xdg
            .filter(|p| p.is_absolute())
            .or_else(|| home.map(|p| p.join(".config")))?
            .join("timeconvert"),
    };
    Some(dir.join("settings.json"))
}
fn settings_path() -> Option<PathBuf> {
    settings_path_for(
        std::env::consts::OS,
        std::env::var_os("APPDATA")
            .filter(|s| !s.is_empty())
            .map(PathBuf::from),
        std::env::var_os("XDG_CONFIG_HOME")
            .filter(|s| !s.is_empty())
            .map(PathBuf::from),
        std::env::var_os("HOME")
            .filter(|s| !s.is_empty())
            .map(PathBuf::from),
    )
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
    /// Row whose timezone picker is open, if any.
    open_picker: Option<usize>,
    /// Bumped every time a picker opens so its popup never reuses a stale size.
    picker_gen: u64,
    settings_open: bool,
    /// AM/PM choice for row A's time field in 12-hour mode.
    pm: bool,
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
            open_picker: None,
            picker_gen: 0,
            settings_open: false,
            pm: false,
        };
        a.refresh();
        a
    }
    fn tz(&self, i: usize) -> Tz {
        self.rows[i].tz
    }
    fn time_text(&self, t: &DateTime<Tz>) -> String {
        if self.settings.time_12h {
            t.format("%-I:%M %p").to_string()
        } else {
            t.format("%H:%M").to_string()
        }
    }
    fn refresh_row(&mut self, i: usize) {
        let t = self.instant.with_timezone(&self.tz(i));
        self.rows[i].date = t.format("%Y-%m-%d").to_string();
        self.rows[i].time = if i == 0 && self.settings.time_12h {
            // Row A is the input: plain "12:45", with AM/PM in its own dropdown.
            self.pm = t.hour() >= 12;
            t.format("%-I:%M").to_string()
        } else {
            self.time_text(&t)
        };
        self.rows[i].message.clear();
    }
    fn refresh(&mut self) {
        for i in 0..self.rows.len() {
            self.refresh_row(i);
        }
    }
    fn convert(&mut self, i: usize) {
        let time = if self.settings.time_12h {
            logic::parse_time_12(&self.rows[i].time, self.pm)
        } else {
            logic::parse_time(&self.rows[i].time)
        };
        let parsed = NaiveDate::parse_from_str(&self.rows[i].date, "%Y-%m-%d")
            .ok()
            .zip(time);
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
                if ui.button("Swap A/B").clicked() {
                    self.settings.zones.swap(0, 1);
                    self.rows.swap(0, 1);
                    if self.reference < 2 {
                        self.reference = 1 - self.reference;
                    }
                    // Re-render row texts so row A's field matches its new zone.
                    self.refresh();
                    save(&self.settings);
                }
                let settings_button = ui.button("Settings");
                if settings_button.clicked() {
                    self.settings_open = !self.settings_open;
                }
                if self.settings_open {
                    let area = egui::Area::new(egui::Id::new("settings_popup"))
                        .order(egui::Order::Foreground)
                        .fixed_pos(settings_button.rect.left_bottom() + egui::vec2(0.0, 4.0))
                        .show(ctx, |ui| {
                            egui::Frame::popup(ui.style()).show(ui, |ui| {
                                ui.set_min_width(200.0);
                                let mut changed = false;
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
                                    changed = true;
                                }
                                if ui
                                    .checkbox(&mut self.settings.analog, "Analog clocks")
                                    .changed()
                                {
                                    changed = true;
                                }
                                if ui
                                    .checkbox(&mut self.settings.time_12h, "12-hour time (AM/PM)")
                                    .changed()
                                {
                                    self.refresh();
                                    changed = true;
                                }
                                if changed {
                                    save(&self.settings);
                                }
                            });
                        });
                    let escape = ctx.input(|i| i.key_pressed(egui::Key::Escape));
                    if escape
                        || (settings_button.clicked_elsewhere()
                            && area.response.clicked_elsewhere())
                    {
                        self.settings_open = false;
                    }
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
                                    let combo_width = (ui.available_width()
                                        - if i >= 2 {
                                            80.0
                                        } else if i == 0 {
                                            62.0
                                        } else {
                                            8.0
                                        })
                                    .clamp(120.0, 310.0);
                                    let (button_rect, button) = ui.allocate_exact_size(
                                        egui::vec2(combo_width, 28.0),
                                        egui::Sense::click(),
                                    );
                                    let is_open = self.open_picker == Some(i);
                                    let fill = if is_open || button.hovered() {
                                        egui::Color32::from_rgb(48, 78, 90)
                                    } else {
                                        egui::Color32::from_rgb(37, 57, 72)
                                    };
                                    ui.painter().rect_filled(
                                        button_rect,
                                        egui::CornerRadius::same(6),
                                        fill,
                                    );
                                    let text_area = egui::Rect::from_min_max(
                                        button_rect.min + egui::vec2(10.0, 0.0),
                                        egui::pos2(button_rect.right() - 26.0, button_rect.bottom()),
                                    );
                                    let galley = ui.painter().layout_no_wrap(
                                        selected.clone(),
                                        egui::FontId::proportional(14.0),
                                        egui::Color32::WHITE,
                                    );
                                    let text_pos = egui::pos2(
                                        text_area.left(),
                                        button_rect.center().y - galley.size().y / 2.0,
                                    );
                                    ui.painter().with_clip_rect(text_area).galley(
                                        text_pos,
                                        galley,
                                        egui::Color32::WHITE,
                                    );
                                    let arrow = egui::pos2(
                                        button_rect.right() - 13.0,
                                        button_rect.center().y,
                                    );
                                    ui.painter().add(egui::Shape::convex_polygon(
                                        vec![
                                            arrow + egui::vec2(-4.5, -2.5),
                                            arrow + egui::vec2(4.5, -2.5),
                                            arrow + egui::vec2(0.0, 3.0),
                                        ],
                                        MUTED,
                                        egui::Stroke::NONE,
                                    ));
                                    if button.clicked() {
                                        if is_open {
                                            self.open_picker = None;
                                        } else {
                                            // A fresh generation gives the popup a fresh id, so
                                            // egui never reuses a size remembered from an earlier
                                            // (differently filtered) list.
                                            self.picker_gen += 1;
                                            self.open_picker = Some(i);
                                            self.rows[i].query.clear();
                                            self.rows[i].popup_open = false;
                                        }
                                    }
                                    if self.open_picker == Some(i) {
                                        let popup_width = button_rect.width().max(300.0);
                                        let mut chosen: Option<Tz> = None;
                                        // The popup has a FIXED height so it never resizes (and
                                        // never gets re-pushed around by egui's keep-inside-the-
                                        // window logic) while the list is filtered. It opens
                                        // downward when there is room, otherwise upward.
                                        let screen = ui.ctx().screen_rect();
                                        let below = screen.bottom() - button_rect.bottom() - 8.0;
                                        let above = button_rect.top() - screen.top() - 8.0;
                                        let chrome = 28.0 + 26.0; // search box + frame padding
                                        let open_down = below >= 190.0 + chrome || below >= above;
                                        let room = if open_down { below } else { above };
                                        let list_height = (room - chrome).clamp(70.0, 190.0);
                                        let popup_area = egui::Area::new(egui::Id::new((
                                            "tz_popup",
                                            i,
                                            self.picker_gen,
                                        )))
                                        .order(egui::Order::Foreground);
                                        let popup_area = if open_down {
                                            popup_area.fixed_pos(
                                                button_rect.left_bottom() + egui::vec2(0.0, 4.0),
                                            )
                                        } else {
                                            popup_area
                                                .pivot(egui::Align2::LEFT_BOTTOM)
                                                .fixed_pos(
                                                    button_rect.left_top() - egui::vec2(0.0, 4.0),
                                                )
                                        };
                                        let area = popup_area.show(ui.ctx(), |ui| {
                                            egui::Frame::popup(ui.style()).show(ui, |ui| {
                                                ui.set_width(popup_width);
                                                ui.style_mut().wrap_mode =
                                                    Some(egui::TextWrapMode::Truncate);
                                                let search = ui.add(
                                                    egui::TextEdit::singleline(
                                                        &mut self.rows[i].query,
                                                    )
                                                    .hint_text("Search country, city, or IANA zone")
                                                    .desired_width(f32::INFINITY),
                                                );
                                                if !self.rows[i].popup_open {
                                                    search.request_focus();
                                                    self.rows[i].popup_open = true;
                                                }
                                                let query = self.rows[i].query.to_lowercase();
                                                let mut first: Option<Tz> = None;
                                                let mut list = egui::ScrollArea::vertical()
                                                    .id_salt(("tz_list", i, self.picker_gen))
                                                    .min_scrolled_height(list_height)
                                                    .max_height(list_height)
                                                    .auto_shrink([false, false]);
                                                if search.changed() {
                                                    // New filter: start from the top of the list.
                                                    list = list.vertical_scroll_offset(0.0);
                                                }
                                                list.show(ui, |ui| {
                                                        for z in self
                                                            .zones
                                                            .iter()
                                                            .filter(|z| logic::matches(z, &query))
                                                        {
                                                            first.get_or_insert(z.tz);
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
                                                                chosen = Some(z.tz);
                                                            }
                                                        }
                                                        if first.is_none() {
                                                            ui.label(
                                                                egui::RichText::new("No matches")
                                                                    .color(MUTED),
                                                            );
                                                        }
                                                    });
                                                if search.lost_focus()
                                                    && ui.input(|inp| {
                                                        inp.key_pressed(egui::Key::Enter)
                                                    })
                                                {
                                                    chosen = chosen.or(first);
                                                }
                                            });
                                        });
                                        let escape =
                                            ui.input(|inp| inp.key_pressed(egui::Key::Escape));
                                        if let Some(tz) = chosen {
                                            self.settings.zones[i] = tz.name().into();
                                            self.rows[i].tz = tz;
                                            self.rows[i].query.clear();
                                            self.rows[i].popup_open = false;
                                            self.open_picker = None;
                                        } else if escape
                                            || (button.clicked_elsewhere()
                                                && area.response.clicked_elsewhere())
                                        {
                                            self.rows[i].popup_open = false;
                                            self.open_picker = None;
                                        }
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
                                                .hint_text(if self.settings.time_12h {
                                                    "h:mm"
                                                } else {
                                                    "HH:MM"
                                                })
                                                .interactive(i == 0),
                                        );
                                        if self.settings.time_12h {
                                            egui::ComboBox::from_id_salt("meridiem")
                                                .selected_text(if self.pm { "PM" } else { "AM" })
                                                .width(34.0)
                                                .show_ui(ui, |ui| {
                                                    ui.selectable_value(&mut self.pm, false, "AM");
                                                    ui.selectable_value(&mut self.pm, true, "PM");
                                                });
                                        }
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
    fn settings_paths() {
        let home = std::env::temp_dir().join("home");
        let xdg = std::env::temp_dir().join("config");
        assert_eq!(
            settings_path_for("linux", None, Some(xdg.clone()), Some(home.clone())),
            Some(xdg.join("timeconvert/settings.json"))
        );
        assert_eq!(
            settings_path_for(
                "linux",
                None,
                Some(PathBuf::from("relative")),
                Some(home.clone())
            ),
            Some(home.join(".config/timeconvert/settings.json"))
        );
        assert_eq!(
            settings_path_for("windows", Some(home.clone()), None, None),
            Some(home.join("TimeConvert/settings.json"))
        );
    }
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
            time_12h: false,
        };
        s.normalize();
        assert_eq!(s.zones.len(), 2);
        s.zones = vec!["UTC".into(); 9];
        s.normalize();
        assert_eq!(s.zones.len(), 9);
    }
    #[test]
    fn old_settings_default_to_24_hour() {
        let s: Settings = serde_json::from_str(r#"{"zones":["UTC","Asia/Tokyo"],"analog":true}"#)
            .unwrap();
        assert!(!s.time_12h);
        let s: Settings = serde_json::from_str(r#"{"time_12h":true}"#).unwrap();
        assert!(s.time_12h);
    }
}
