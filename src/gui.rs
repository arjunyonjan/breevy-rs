use eframe::egui;
use egui::{Color32, Frame, Margin, Rounding, Stroke, Vec2};

use breevy_core::{parse_bvy, remove_bvy_record, update_bvy_record, Snippet};

const BG: Color32 = Color32::from_rgb(0xf0, 0xf0, 0xf0);
const DARK: Color32 = Color32::from_rgb(0x1a, 0x1a, 0x1a);
const SEL: Color32 = Color32::from_rgb(0xcc, 0xe8, 0xff);
const SEL_STROKE: Color32 = Color32::from_rgb(0xa8, 0xc8, 0xe8);
const FOLDER: Color32 = Color32::from_rgb(0xf2, 0xc1, 0x4e);
const FOLDER_EDGE: Color32 = Color32::from_rgb(0xc7, 0x9a, 0x2c);
const GREEN: Color32 = Color32::from_rgb(0x3a, 0xa6, 0x3a);

pub fn run(path: String) -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([812.0, 614.0])
            .with_min_inner_size([640.0, 480.0])
            .with_title("Breevy"),
        ..Default::default()
    };
    eframe::run_native(
        "Breevy",
        options,
        Box::new(move |cc| Ok(Box::new(BreevyApp::new(cc, path)))),
    )
}

struct BreevyApp {
    path: String,
    raw: String,
    snippets: Vec<Snippet>,
    folders: Vec<String>,
    selected: Option<usize>,
    filter: String,
    edit_abbr: String,
    edit_exp: String,
    edit_label: String,
    edit_case: bool,
    status: String,
    dirty: bool,
    styled: bool,
    show_prefs: bool,
    trigger: String,
    new_folder: Option<String>,
}

impl BreevyApp {
    fn new(cc: &eframe::CreationContext<'_>, path: String) -> Self {
        style(&cc.egui_ctx);
        let (snippets, folders, raw) = load(&path);
        let mut app = Self {
            path,
            raw,
            snippets,
            folders,
            selected: None,
            filter: String::new(),
            edit_abbr: String::new(),
            edit_exp: String::new(),
            edit_label: String::new(),
            edit_case: false,
            status: "Status: Enabled".to_string(),
            dirty: false,
            styled: true,
            show_prefs: false,
            trigger: load_trigger(),
            new_folder: None,
        };
        if !app.snippets.is_empty() {
            app.select(0);
        }
        app
    }

    fn select(&mut self, idx: usize) {
        if let Some(s) = self.snippets.get(idx) {
            self.selected = Some(idx);
            self.edit_abbr = s.abbreviation.clone();
            self.edit_exp = s.expansion.clone();
            self.edit_label = s.label.clone();
            self.edit_case = s.case_sensitive;
        }
    }

    fn apply(&mut self) {
        let Some(idx) = self.selected else { return };
        if let Some(s) = self.snippets.get_mut(idx) {
            s.abbreviation = self.edit_abbr.clone();
            s.expansion = self.edit_exp.clone();
            s.label = self.edit_label.clone();
            s.case_sensitive = self.edit_case;
            s.mode = 2;
            let updated = s.clone();
            self.raw = update_bvy_record(&self.raw, &updated);
            match std::fs::write(&self.path, &self.raw) {
                Ok(_) => {
                    self.status = format!("Status: Saved {} snippets", self.snippets.len());
                    self.dirty = false;
                }
                Err(e) => self.status = format!("Status: save failed: {e}"),
            }
        }
    }

    fn dialogs(&mut self, ctx: &egui::Context) {
        if self.new_folder.is_some() {
            let mut open = true;
            let mut name = self.new_folder.clone().unwrap_or_default();
            egui::Window::new("New Folder")
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label("Folder name");
                    ui.text_edit_singleline(&mut name);
                    ui.horizontal(|ui| {
                        if ui.button("Create").clicked() && !name.trim().is_empty() {
                            let fname = name.trim().to_string();
                            let xml = format!("<f n=\"{}\"></f>", xml_attr(&fname));
                            if let Some(pos) = self.raw.rfind("</Breevy>") {
                                self.raw.insert_str(pos, &xml);
                            } else {
                                self.raw.push_str(&xml);
                            }
                            self.folders.push(fname.clone());
                            let _ = std::fs::write(&self.path, &self.raw);
                            self.status = format!("Status: folder '{fname}' added");
                            self.new_folder = None;
                        }
                        if ui.button("Cancel").clicked() {
                            self.new_folder = None;
                        }
                    });
                });
            if !open {
                self.new_folder = None;
            }
        }

        if self.show_prefs {
            let mut open = true;
            egui::Window::new("Preferences")
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label("Trigger key (for trigger-mode snippets)");
                    egui::ComboBox::from_id_salt("prefs_trigger")
                        .selected_text(self.trigger.clone())
                        .show_ui(ui, |ui| {
                            for t in ["ctrl", "alt", "shift"] {
                                ui.selectable_value(&mut self.trigger, t.to_string(), t);
                            }
                        });
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            save_trigger(&self.trigger);
                            self.status = "Status: preferences saved, restart hook".into();
                        }
                    });
                });
            if !open {
                self.show_prefs = false;
            }
        }
    }

    fn filtered(&self) -> Vec<usize> {
        let q = self.filter.trim().to_lowercase();
        (0..self.snippets.len())
            .filter(|&i| {
                if q.is_empty() {
                    return true;
                }
                let s = &self.snippets[i];
                s.abbreviation.to_lowercase().contains(&q)
                    || s.label.to_lowercase().contains(&q)
                    || s.expansion.to_lowercase().contains(&q)
            })
            .collect()
    }
}

impl eframe::App for BreevyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.styled {
            style(ctx);
            self.styled = true;
        }

        egui::TopBottomPanel::top("menu")
            .exact_height(20.0)
            .frame(panel_frame())
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 14.0;
                    for m in ["File", "Options", "Backup/Manager", "Help"] {
                        let resp = ui.add(
                            egui::Label::new(egui::RichText::new(m).size(11.0).color(DARK))
                                .sense(egui::Sense::click()),
                        );
                        if m == "Options" && resp.clicked() {
                            self.show_prefs = true;
                        }
                    }
                });
            });

        egui::TopBottomPanel::top("toolbar")
            .exact_height(32.0)
            .frame(panel_frame())
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    if tb(ui, "New Folder", FOLDER).clicked() {
                        self.new_folder = Some(String::new());
                    }
                    if tb(ui, "New Abbreviation", Color32::from_rgb(0x4a, 0x90, 0xd9)).clicked() {
                        let n = self.snippets.len() + 1;
                        self.snippets.push(Snippet {
                            abbreviation: format!("new{n}"),
                            expansion: String::new(),
                            label: "New abbreviation".into(),
                            mode: 2,
                            snippet_type: 0,
                            uuid: format!("breevy-rs-{n:08x}"),
                            case_sensitive: false,
                        });
                        let idx = self.snippets.len() - 1;
                        let new_snip = self.snippets[idx].clone();
                        self.raw = update_bvy_record(&self.raw, &new_snip);
                        self.select(idx);
                        self.dirty = true;
                    }
                    if tb(ui, "New Launcher", Color32::from_rgb(0x4a, 0x90, 0xd9)).clicked() {
                        let n = self.snippets.len() + 1;
                        self.snippets.push(Snippet {
                            abbreviation: format!("launch{n}"),
                            expansion: "https://".into(),
                            label: "New launcher".into(),
                            mode: 2,
                            snippet_type: 1,
                            uuid: format!("breevy-rs-launch-{n:08x}"),
                            case_sensitive: false,
                        });
                        let idx = self.snippets.len() - 1;
                        let new_snip = self.snippets[idx].clone();
                        self.raw = update_bvy_record(&self.raw, &new_snip);
                        self.select(idx);
                        self.dirty = true;
                        self.status = "Status: launcher added, set the target URL".into();
                    }
                    ui.add_space(4.0);
                    ui.separator();
                    let del = ui.add(
                        egui::Button::new(egui::RichText::new(" ✕ ").color(Color32::WHITE))
                            .fill(Color32::from_rgb(0xd9, 0x4a, 0x4a))
                            .rounding(2.0),
                    );
                    if del.clicked() {
                        if let Some(idx) = self.selected {
                            if idx < self.snippets.len() {
                                let uuid = self.snippets[idx].uuid.clone();
                                self.raw = remove_bvy_record(&self.raw, &uuid);
                                self.snippets.remove(idx);
                                self.selected = None;
                                self.dirty = true;
                                self.status = "Status: deleted".into();
                                if !self.snippets.is_empty() {
                                    self.select(0);
                                } else {
                                    self.edit_abbr.clear();
                                    self.edit_exp.clear();
                                    self.edit_label.clear();
                                }
                            }
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.filter)
                                .hint_text("Search...")
                                .desired_width(150.0)
                                .font(egui::TextStyle::Body),
                        );
                    });
                });
            });

        egui::TopBottomPanel::bottom("status")
            .exact_height(26.0)
            .frame(panel_frame())
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(10.0), egui::Sense::hover());
                    ui.painter().circle_filled(rect.center(), 5.0, GREEN);
                    ui.label(egui::RichText::new(&self.status).size(11.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(btn("Apply", true)).clicked() {
                            self.apply();
                        }
                        if ui.add(btn("Cancel", false)).clicked() {
                            if let Some(idx) = self.selected {
                                self.select(idx);
                            }
                            self.status = "Status: Enabled".into();
                        }
                        if ui.add(btn("OK", false)).clicked() {
                            self.apply();
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                });
            });

        egui::SidePanel::left("tree")
            .exact_width(196.0)
            .resizable(false)
            .frame(Frame::none().fill(Color32::WHITE).inner_margin(Margin::symmetric(4.0, 4.0)))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 1.0;
                folder_node(ui, "▾", "All Abbreviations / Folders", true, true, DARK);
                folder_node(ui, "", "All Abbreviations", false, false, DARK);
                for f in &self.folders {
                    folder_node(ui, "", f, false, false, DARK);
                }
                ui.allocate_space(ui.available_size());
            });

        egui::CentralPanel::default()
            .frame(Frame::none().fill(BG))
            .show(ctx, |ui| {
                egui::TopBottomPanel::top("list")
                    .exact_height(242.0)
                    .frame(Frame::none().fill(Color32::WHITE))
                    .show_inside(ui, |ui| {
                        ui.spacing_mut().item_spacing = Vec2::ZERO;
                        let w = ui.available_width();
                        let c1 = 150.0;
                        let c2 = 110.0;
                        let c3 = (w - c1 - c2).max(80.0);
                        ui.horizontal(|ui| {
                            let (r, _) = ui.allocate_exact_size(Vec2::new(c1, 18.0), egui::Sense::hover());
                            ui.painter().rect_filled(r, 0.0, BG);
                            ui.painter().text(
                                egui::pos2(r.left() + 6.0, r.center().y),
                                egui::Align2::LEFT_CENTER,
                                "Description",
                                egui::FontId::proportional(11.0),
                                DARK,
                            );
                            let (r, _) = ui.allocate_exact_size(Vec2::new(c2, 18.0), egui::Sense::hover());
                            ui.painter().rect_filled(r, 0.0, BG);
                            ui.painter().text(
                                egui::pos2(r.left() + 6.0, r.center().y),
                                egui::Align2::LEFT_CENTER,
                                "Abbreviation",
                                egui::FontId::proportional(11.0),
                                DARK,
                            );
                            let (r, _) = ui.allocate_exact_size(Vec2::new(c3, 18.0), egui::Sense::hover());
                            ui.painter().rect_filled(r, 0.0, BG);
                            ui.painter().text(
                                egui::pos2(r.left() + 6.0, r.center().y),
                                egui::Align2::LEFT_CENTER,
                                "Replacement Preview",
                                egui::FontId::proportional(11.0),
                                DARK,
                            );
                        });
                        ui.painter().hline(
                            ui.min_rect().x_range(),
                            ui.min_rect().top() + 18.0,
                            Stroke::new(1.0, Color32::from_rgb(0xc8, 0xc8, 0xc8)),
                        );

                        let list = self.filtered();
                        let row_h = 19.0;
                        let mut clicked: Option<usize> = None;
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show_rows(ui, row_h, list.len(), |ui, range| {
                                for li in range {
                                    let idx = list[li];
                                    let s = &self.snippets[idx];
                                    let (rect, resp) = ui.allocate_exact_size(
                                        Vec2::new(ui.available_width(), row_h),
                                        egui::Sense::click(),
                                    );
                                    let sel = self.selected == Some(idx);
                                    if sel {
                                        ui.painter().rect_filled(rect, 0.0, SEL);
                                        ui.painter().rect_stroke(
                                            rect,
                                            0.0,
                                            Stroke::new(1.0, SEL_STROKE),
                                        );
                                    }
                                    let y = rect.center().y;
                                    ui.painter().text(
                                        egui::pos2(rect.left() + 8.0, y),
                                        egui::Align2::LEFT_CENTER,
                                        &s.label,
                                        egui::FontId::proportional(11.0),
                                        DARK,
                                    );
                                    ui.painter().text(
                                        egui::pos2(rect.left() + 150.0 + 6.0, y),
                                        egui::Align2::LEFT_CENTER,
                                        &s.abbreviation,
                                        egui::FontId::proportional(11.0),
                                        DARK,
                                    );
                                    let preview = s.expansion.lines().next().unwrap_or("");
                                    ui.painter().text(
                                        egui::pos2(rect.left() + 260.0 + 6.0, y),
                                        egui::Align2::LEFT_CENTER,
                                        preview,
                                        egui::FontId::proportional(11.0),
                                        Color32::from_rgb(0x33, 0x33, 0x33),
                                    );
                                    paint_check(ui, rect.right() - 19.0, y);
                                    if resp.clicked() {
                                        clicked = Some(idx);
                                    }
                                }
                            });
                        if let Some(idx) = clicked {
                            self.select(idx);
                        }
                    });

                egui::CentralPanel::default()
                    .frame(Frame::none().fill(BG).inner_margin(Margin::symmetric(8.0, 6.0)))
                    .show_inside(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 3.0;
                        ui.label(egui::RichText::new("Abbreviation").strong().size(11.0));
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.edit_abbr)
                                    .desired_width(180.0)
                                    .font(egui::TextStyle::Body),
                            );
                            egui::ComboBox::from_id_salt("case")
                                .width(ui.available_width() - 4.0)
                                .selected_text(if self.edit_case {
                                    "Case sensitive (typing 'AB' matches 'AB')"
                                } else {
                                    "Case insensitive (typing 'AB' matches 'ab', 'Ab'...)"
                                })
                                .show_ui(ui, |ui| {
                                    let before = self.edit_case;
                                    ui.selectable_value(
                                        &mut self.edit_case,
                                        false,
                                        "Case insensitive (typing 'AB' matches 'ab', 'Ab'...)",
                                    );
                                    ui.selectable_value(
                                        &mut self.edit_case,
                                        true,
                                        "Case sensitive (typing 'AB' matches 'AB')",
                                    );
                                    if before != self.edit_case {
                                        self.dirty = true;
                                    }
                                });
                        });
                        ui.label(egui::RichText::new("Replacement Text").strong().size(11.0));
                        ui.add_sized(
                            [ui.available_width(), 78.0],
                            egui::TextEdit::multiline(&mut self.edit_exp)
                                .font(egui::TextStyle::Body)
                                .desired_rows(4),
                        );
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Replace Typed abbreviation:").size(11.0));
                            egui::ComboBox::from_id_salt("trigger")
                                .selected_text("immediately")
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut (), (), "immediately");
                                });
                        });
                        ui.label(egui::RichText::new("Description").strong().size(11.0));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.edit_label)
                                .desired_width(ui.available_width())
                                .font(egui::TextStyle::Body),
                        );
                    });
            });
        self.dialogs(ctx);
    }
}

fn style(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::light();
    visuals.panel_fill = BG;
    visuals.window_fill = BG;
    visuals.extreme_bg_color = Color32::WHITE;
    visuals.faint_bg_color = Color32::from_rgb(0xf5, 0xf5, 0xf5);
    visuals.selection.bg_fill = SEL;
    visuals.selection.stroke = Stroke::new(1.0, SEL_STROKE);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, DARK);
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(0xf5, 0xf5, 0xf5);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0x8a, 0x8a, 0x8a));
    visuals.widgets.inactive.rounding = Rounding::same(2.0);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(0xe6, 0xf0, 0xfa);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0xa8, 0xc8, 0xe8));
    visuals.widgets.active.bg_fill = Color32::from_rgb(0xdc, 0xec, 0xff);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0x6a, 0x9e, 0xd0));
    let mut style = (*ctx.style()).clone();
    style.visuals = visuals;
    style.spacing.item_spacing = Vec2::new(4.0, 2.0);
    style.spacing.button_padding = Vec2::new(6.0, 2.0);
    style.text_styles = [
        (egui::TextStyle::Body, egui::FontId::proportional(11.0)),
        (egui::TextStyle::Button, egui::FontId::proportional(11.0)),
        (egui::TextStyle::Heading, egui::FontId::proportional(12.0)),
        (egui::TextStyle::Monospace, egui::FontId::monospace(10.0)),
        (egui::TextStyle::Small, egui::FontId::proportional(9.0)),
    ]
    .into();
    ctx.set_style(style);
}

fn panel_frame() -> Frame {
    Frame::none()
        .fill(BG)
        .inner_margin(Margin::symmetric(6.0, 0.0))
        .stroke(Stroke::new(0.0, Color32::TRANSPARENT))
}

fn btn(text: &str, primary: bool) -> egui::Button<'static> {
    let b = egui::Button::new(egui::RichText::new(text.to_string()).size(11.0)).rounding(3.0);
    if primary {
        b.fill(Color32::from_rgb(0xbc, 0xd8, 0xf8))
            .stroke(Stroke::new(1.0, Color32::from_rgb(0x6a, 0x9e, 0xd0)))
    } else {
        b.fill(Color32::from_rgb(0xe4, 0xe4, 0xe4))
            .stroke(Stroke::new(1.0, Color32::from_rgb(0x8a, 0x8a, 0x8a)))
    }
}

fn tb(ui: &mut egui::Ui, text: &str, icon: Color32) -> egui::Response {
    let resp = ui
        .add(egui::Button::new(egui::RichText::new(text).size(11.0)).rounding(3.0))
        .on_hover_text(text);
    let rect = resp.rect;
    let icon_rect = egui::Rect::from_min_size(
        egui::pos2(rect.left() + 3.0, rect.center().y - 5.0),
        Vec2::splat(10.0),
    );
    ui.painter().rect_filled(icon_rect, 2.0, icon);
    resp
}

fn folder_node(ui: &mut egui::Ui, arrow: &str, name: &str, root: bool, selected: bool, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), 17.0),
        egui::Sense::hover(),
    );
    if selected {
        ui.painter().rect_filled(rect, 1.0, SEL);
    }
    let y = rect.center().y;
    if !arrow.is_empty() {
        ui.painter().text(
            egui::pos2(rect.left() + 2.0, y),
            egui::Align2::LEFT_CENTER,
            arrow,
            egui::FontId::proportional(9.0),
            Color32::from_rgb(0x66, 0x66, 0x66),
        );
    }
    let icon = egui::Rect::from_min_size(
        egui::pos2(rect.left() + 14.0, y - 6.0),
        Vec2::splat(12.0),
    );
    ui.painter().rect_filled(icon, 1.0, FOLDER);
    ui.painter().rect_stroke(icon, 1.0, Stroke::new(1.0, FOLDER_EDGE));
    ui.painter().text(
        egui::pos2(rect.left() + 31.0, y),
        egui::Align2::LEFT_CENTER,
        name,
        egui::FontId::proportional(if root { 11.0 } else { 11.0 }),
        color,
    );
    if root {
        let w = ui.painter().layout_no_wrap(
            name.to_string(),
            egui::FontId::proportional(11.0),
            color,
        );
        let _ = w;
    }
}

fn paint_check(ui: &egui::Ui, right: f32, y: f32) {
    let rect = egui::Rect::from_center_size(egui::pos2(right, y), Vec2::splat(13.0));
    ui.painter().rect_filled(rect, 2.0, GREEN);
    let p = |dx: f32, dy: f32| egui::pos2(rect.left() + dx, rect.top() + dy);
    ui.painter()
        .line_segment([p(3.0, 6.5), p(5.5, 9.0)], Stroke::new(1.6, Color32::WHITE));
    ui.painter()
        .line_segment([p(5.5, 9.0), p(10.0, 3.5)], Stroke::new(1.6, Color32::WHITE));
}

fn config_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    std::path::Path::new(&home).join(".config/breevy/config")
}

fn load_trigger() -> String {
    std::fs::read_to_string(config_path())
        .ok()
        .and_then(|c| {
            c.lines()
                .find_map(|l| l.strip_prefix("trigger=").map(|v| v.trim().to_string()))
        })
        .unwrap_or_else(|| "ctrl".to_string())
}

fn save_trigger(t: &str) {
    let p = config_path();
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(p, format!("trigger={t}\n"));
}

fn xml_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn load(path: &str) -> (Vec<Snippet>, Vec<String>, String) {
    let Ok(text) = std::fs::read_to_string(path) else {
        return (Vec::new(), Vec::new(), String::new());
    };
    let snippets = parse_bvy(&text);
    let folders = extract_folders(&text);
    (snippets, folders, text)
}

fn is_uuid(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 36
        && b.iter().enumerate().all(|(i, c)| {
            if i == 8 || i == 13 || i == 18 || i == 23 {
                *c == b'-'
            } else {
                c.is_ascii_hexdigit()
            }
        })
}

fn extract_folders(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut push = |name: &str| {
        let name = name.trim();
        if !name.is_empty() && !is_uuid(name) && !out.iter().any(|x| x == name) {
            out.push(name.to_string());
        }
    };
    let mut rest = text;
    while let Some(pos) = rest.find("<f n=\"") {
        rest = &rest[pos + 6..];
        if let Some(e) = rest.find('"') {
            push(&rest[..e]);
        }
    }
    let mut rest = text;
    while let Some(pos) = rest.find("<key>groupName</key>") {
        rest = &rest[pos + 21..];
        let trimmed = rest.trim_start();
        if let Some(after) = trimmed.strip_prefix("<string>") {
            if let Some(e) = after.find("</string>") {
                push(&after[..e]);
            }
        }
    }
    out
}
