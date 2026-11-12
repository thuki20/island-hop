#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! IslandHop — 屏幕顶部的一条「灵动岛」，把局域网键鼠 / 剪贴板共享收进里面。
//! 引擎复用 openhop-core，UI 用 egui 自绘，不依赖 WebView2 / Electron。

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, Frame, Stroke, StrokeKind,
    Vec2, ViewportBuilder, ViewportCommand,
};
use openhop_core::platform;
use openhop_core::{Config, Engine, Role, Status};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

const W_SMALL: f32 = 236.0;
const H_SMALL: f32 = 40.0;
const W_BIG: f32 = 452.0;
const H_BIG: f32 = 372.0;

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let (x, y) = top_center(W_SMALL);
    let opts = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size([W_SMALL, H_SMALL])
            .with_min_inner_size([W_SMALL, H_SMALL])
            .with_position([x, y])
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_resizable(false),
        ..Default::default()
    };
    eframe::run_native(
        "IslandHop",
        opts,
        Box::new(|cc| {
            let cjk = setup_fonts(&cc.egui_ctx);
            Ok(Box::new(IslandApp::new(cjk)) as Box<dyn eframe::App>)
        }),
    )
}

fn top_center(w: f32) -> (f32, f32) {
    match platform::monitors().first() {
        Some(m) => ((m.x + (m.w - w as i32) / 2) as f32, (m.y + 6) as f32),
        None => (400.0, 20.0),
    }
}

/// egui 默认字体不含中文，运行时挂系统字体；失败则 UI 文案降级为英文。
fn setup_fonts(ctx: &egui::Context) -> bool {
    let paths: &[&str] = if cfg!(windows) {
        &[
            "C:\\Windows\\Fonts\\msyh.ttc",
            "C:\\Windows\\Fonts\\msyhei.ttf",
            "C:\\Windows\\Fonts\\simhei.ttf",
        ]
    } else if cfg!(target_os = "macos") {
        &[
            "/System/Library/Fonts/PingFang.ttc",
            "/System/Library/Fonts/STHeiti Medium.ttc",
            "/System/Library/Fonts/Hiragino Sans GB.ttc",
        ]
    } else {
        &[]
    };
    for p in paths {
        if let Ok(bytes) = fs::read(p) {
            let mut fonts = FontDefinitions::default();
            fonts
                .font_data
                .insert("cjk".to_owned(), Arc::new(FontData::from_owned(bytes)));
            fonts
                .families
                .entry(FontFamily::Proportional)
                .or_default()
                .insert(0, "cjk".to_owned());
            fonts
                .families
                .entry(FontFamily::Monospace)
                .or_default()
                .insert(0, "cjk".to_owned());
            ctx.set_fonts(fonts);
            return true;
        }
    }
    log::warn!("no CJK system font found; UI falls back to English labels");
    false
}

fn config_path() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join("island-hop");
    let _ = fs::create_dir_all(&dir);
    dir.join("config.toml")
}

fn t(cjk: bool, zh: &'static str, en: &'static str) -> &'static str {
    if cjk { zh } else { en }
}

fn clock() -> String {
    chrono::Local::now().format("%H:%M").to_string()
}

enum Action {
    Role(Role),
    SaveName,
    SavePass,
    Pair(String, String),
    Place(String, i32),
    Clipboard(bool),
    Restart,
    Quit,
}

struct IslandApp {
    cjk: bool,
    cfg_path: PathBuf,
    cfg: Config,
    engine: Option<Engine>,
    status: Option<Status>,
    last_poll: Instant,
    last_hover: Instant,
    expanded: bool,
    pair_code: String,
    name_buf: String,
    pass_buf: String,
    permission_note: Option<String>,
    error: Option<String>,
    quit: bool,
}

impl IslandApp {
    fn new(cjk: bool) -> Self {
        let cfg_path = config_path();
        // 从旧 OpenHop 继承配对关系 / 布局 / 口令，重装后不必重新配对。
        if !cfg_path.exists() {
            let legacy = Config::default_path();
            if legacy.exists() {
                if let Ok(old) = Config::load(&legacy) {
                    let _ = old.save(&cfg_path);
                    log::info!("imported settings from {}", legacy.display());
                }
            }
        }
        let cfg = Config::load(&cfg_path).unwrap_or_default();
        let name_buf = cfg.name.clone();
        let pass_buf = cfg.passphrase.clone();
        let mut app = IslandApp {
            cjk,
            cfg_path,
            cfg,
            engine: None,
            status: None,
            last_poll: Instant::now() - Duration::from_secs(2),
            last_hover: Instant::now() - Duration::from_secs(2),
            expanded: false,
            pair_code: String::new(),
            name_buf,
            pass_buf,
            permission_note: None,
            error: None,
            quit: false,
        };
        app.restart();
        app
    }

    fn restart(&mut self) {
        if let Some(e) = self.engine.take() {
            e.stop();
        }
        self.permission_note = platform::check_permissions(true);
        match Engine::start(self.cfg.clone(), Some(self.cfg_path.clone())) {
            Ok(e) => {
                self.engine = Some(e);
                self.error = None;
            }
            Err(err) => self.error = Some(err.to_string()),
        }
        self.last_poll = Instant::now() - Duration::from_secs(2);
    }

    fn apply(&mut self, action: Action) {
        match action {
            Action::Role(r) => {
                self.cfg.role = r;
                let _ = self.cfg.save(&self.cfg_path);
                self.restart();
            }
            Action::SaveName => {
                self.cfg.name = self.name_buf.trim().to_string();
                let _ = self.cfg.save(&self.cfg_path);
                self.restart();
            }
            Action::SavePass => {
                self.cfg.passphrase = self.pass_buf.trim().to_string();
                let _ = self.cfg.save(&self.cfg_path);
                self.restart();
            }
            Action::Pair(device, code) => {
                if let Some(e) = &self.engine {
                    e.pair(device, code);
                }
            }
            Action::Place(name, x) => {
                self.cfg.layout.place(&name, x, 0);
                let _ = self.cfg.save(&self.cfg_path);
                if let Some(e) = &self.engine {
                    e.set_layout(self.cfg.layout.clone());
                }
            }
            Action::Clipboard(on) => {
                self.cfg.clipboard_sync = on;
                let _ = self.cfg.save(&self.cfg_path);
                self.restart();
            }
            Action::Restart => self.restart(),
            Action::Quit => self.quit = true,
        }
    }

    fn dot(&self) -> (Color32, &'static str) {
        let c = self.cjk;
        match &self.status {
            Some(s) if !s.running || s.error.is_some() => (
                Color32::from_rgb(230, 90, 90),
                t(c, "已停止", "Stopped"),
            ),
            Some(s) if s.role == Role::Server && !s.active.is_empty() => (
                Color32::from_rgb(90, 205, 160),
                t(c, "跨屏中", "Remote"),
            ),
            Some(_) => (
                Color32::from_rgb(110, 200, 245),
                t(c, "已就绪", "Ready"),
            ),
            None => (Color32::from_gray(150), t(c, "启动中", "Starting")),
        }
    }

    fn collapsed(&self, ui: &mut egui::Ui) {
        let (color, label) = self.dot();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 7.0;
            ui.label(egui::RichText::new("●").size(10.0).color(color));
            ui.label(
                egui::RichText::new(&self.cfg.name)
                    .size(13.0)
                    .strong()
                    .color(Color32::WHITE),
            );
            ui.label(
                egui::RichText::new(label)
                    .size(11.0)
                    .color(Color32::from_gray(165)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(clock())
                        .size(11.0)
                        .color(Color32::from_gray(200)),
                );
            });
        });
    }

    fn expanded_ui(&mut self, ui: &mut egui::Ui) {
        let c = self.cjk;
        let mut act: Option<Action> = None;
        let mut pair_code = std::mem::take(&mut self.pair_code);
        let mut name_buf = std::mem::take(&mut self.name_buf);
        let mut pass_buf = std::mem::take(&mut self.pass_buf);

        {
            let s = self.status.as_ref();
            let (dot_color, state_label) = self.dot();
            let is_server = self.cfg.role == Role::Server;
            let muted = Color32::from_gray(175);

            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("IslandHop")
                        .size(14.0)
                        .strong()
                        .color(Color32::WHITE),
                );
                ui.label(egui::RichText::new("●").size(9.0).color(dot_color));
                ui.label(egui::RichText::new(state_label).size(11.0).color(muted));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(clock()).size(11.0).color(muted));
                });
            });
            ui.add_space(5.0);
            ui.separator();
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(t(c, "角色", "Role")).size(12.0).color(muted));
                if ui.selectable_label(is_server, t(c, "主控（共享键鼠）", "Server")).clicked()
                    && !is_server
                {
                    act = Some(Action::Role(Role::Server));
                }
                if ui.selectable_label(!is_server, t(c, "受控", "Client")).clicked() && is_server {
                    act = Some(Action::Role(Role::Client));
                }
            });
            ui.add_space(3.0);
            let message = s.map(|x| x.message.as_str()).unwrap_or("");
            ui.label(egui::RichText::new(message).size(11.0).color(Color32::from_gray(150)));
            if let Some(e) = &self.error {
                ui.label(egui::RichText::new(e).size(11.0).color(Color32::from_rgb(240, 130, 120)));
            }
            if let Some(p) = &self.permission_note {
                ui.label(egui::RichText::new(p).size(11.0).color(Color32::from_rgb(240, 190, 110)));
            }

            ui.add_space(5.0);
            if is_server {
                if let Some(code) = s.and_then(|x| x.pairing_code.as_ref()) {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(t(c, "配对码", "Pair code")).size(12.0).color(muted));
                        ui.label(
                            egui::RichText::new(code.as_str())
                                .size(18.0)
                                .strong()
                                .color(Color32::from_rgb(120, 220, 180)),
                        );
                        ui.label(
                            egui::RichText::new(t(c, "在另一台输入", "enter on the other computer"))
                                .size(11.0)
                                .color(Color32::from_gray(145)),
                        );
                    });
                }
            } else {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(t(c, "配对码", "Pair code")).size(12.0).color(muted));
                    ui.add(
                        egui::TextEdit::singleline(&mut pair_code)
                            .desired_width(90.0)
                            .hint_text("000000"),
                    );
                });
                let servers: Vec<(String, String)> = s
                    .map(|x| {
                        x.discovered
                            .iter()
                            .filter(|d| d.role == Role::Server)
                            .map(|d| (d.device.clone(), d.name.clone()))
                            .collect()
                    })
                    .unwrap_or_default();
                if servers.is_empty() {
                    ui.label(
                        egui::RichText::new(t(c, "尚未发现主控端", "No server discovered yet"))
                            .size(11.0)
                            .color(Color32::from_gray(145)),
                    );
                }
                for (device, name) in servers {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(name).size(12.0).color(Color32::WHITE));
                        if ui.small_button(t(c, "配对", "Pair")).clicked()
                            && !pair_code.trim().is_empty()
                        {
                            act = Some(Action::Pair(device.clone(), pair_code.trim().to_string()));
                        }
                    });
                }
                if let Some(e) = s.and_then(|x| x.pair_error.as_ref()) {
                    ui.label(egui::RichText::new(e).size(11.0).color(Color32::from_rgb(240, 130, 120)));
                }
            }

            ui.add_space(5.0);
            ui.label(
                egui::RichText::new(t(c, "屏幕排列", "Screen layout"))
                    .size(12.0)
                    .color(muted),
            );
            let peers: Vec<(String, i32)> = s
                .map(|x| {
                    x.paired
                        .iter()
                        .map(|p| {
                            (
                                p.name.clone(),
                                x.layout.position(&p.name).map(|v| v.0).unwrap_or(0),
                            )
                        })
                        .collect()
                })
                .unwrap_or_default();
            if peers.is_empty() {
                ui.label(
                    egui::RichText::new(t(c, "尚无已配对设备", "No paired computers"))
                        .size(11.0)
                        .color(Color32::from_gray(145)),
                );
            }
            for (name, x) in peers {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("{}  (x={})", name, x))
                            .size(11.0)
                            .color(Color32::from_gray(210)),
                    );
                    if ui.small_button(t(c, "设为左", "Left")).clicked() {
                        act = Some(Action::Place(name.clone(), -1));
                    }
                    if ui.small_button(t(c, "设为右", "Right")).clicked() {
                        act = Some(Action::Place(name.clone(), 1));
                    }
                });
            }

            ui.add_space(5.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(t(c, "设备名", "Name")).size(12.0).color(muted));
                ui.add(egui::TextEdit::singleline(&mut name_buf).desired_width(112.0));
                if ui.small_button(t(c, "保存", "Save")).clicked() {
                    act = Some(Action::SaveName);
                }
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(t(c, "共享口令", "Passphrase")).size(12.0).color(muted));
                ui.add(egui::TextEdit::singleline(&mut pass_buf).password(true).desired_width(112.0));
                if ui.small_button(t(c, "保存", "Save")).clicked() {
                    act = Some(Action::SavePass);
                }
            });
            let mut clip = self.cfg.clipboard_sync;
            if ui.checkbox(&mut clip, t(c, "剪贴板跨机同步", "Clipboard sync")).changed() {
                act = Some(Action::Clipboard(clip));
            }

            ui.add_space(5.0);
            ui.horizontal(|ui| {
                if ui.small_button(t(c, "重启引擎", "Restart")).clicked() {
                    act = Some(Action::Restart);
                }
                if ui.small_button(t(c, "退出", "Quit")).clicked() {
                    act = Some(Action::Quit);
                }
                if let Some(st) = s {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!("{} · :{}", st.os.label(), st.port))
                                .size(11.0)
                                .color(Color32::from_gray(140)),
                        );
                    });
                }
            });
        }

        self.pair_code = pair_code;
        self.name_buf = name_buf;
        self.pass_buf = pass_buf;
        if let Some(a) = act {
            self.apply(a);
        }
    }
}

impl eframe::App for IslandApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_millis(150));
        if self.last_poll.elapsed() > Duration::from_millis(250) {
            if let Some(e) = &self.engine {
                self.status = Some(e.status());
            }
            self.last_poll = Instant::now();
        }

        let hovering = ctx.input(|i| i.pointer.hover_pos().is_some());
        if hovering {
            self.last_hover = Instant::now();
        }
        let want = if self.expanded {
            self.last_hover.elapsed() < Duration::from_millis(700)
        } else {
            hovering
        };
        if want != self.expanded {
            self.expanded = want;
            let w = if want { W_BIG } else { W_SMALL };
            let h = if want { H_BIG } else { H_SMALL };
            ctx.send_viewport_cmd(ViewportCommand::InnerSize(Vec2::new(w, h)));
            let (x, y) = top_center(w);
            ctx.send_viewport_cmd(ViewportCommand::OuterPosition(egui::pos2(x, y)));
        }
        if self.quit {
            if let Some(e) = self.engine.take() {
                e.stop();
            }
            ctx.send_viewport_cmd(ViewportCommand::Close);
            return;
        }

        let expanded = self.expanded;
        egui::CentralPanel::default()
            .frame(Frame::default().fill(Color32::TRANSPARENT))
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                let r: u8 = if expanded { 16 } else { 20 };
                ui.painter().rect_filled(
                    rect,
                    CornerRadius::same(r),
                    Color32::from_rgba_unmultiplied(24, 25, 29, 240),
                );
                ui.painter().rect_stroke(
                    rect,
                    CornerRadius::same(r),
                    Stroke::new(0.5_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 30)),
                    StrokeKind::Outside,
                );
                let inner = rect.shrink2(Vec2::new(14.0, 9.0));
                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(inner), |ui| {
                    if expanded {
                        self.expanded_ui(ui);
                    } else {
                        self.collapsed(ui);
                    }
                });
            });
    }
}

impl Drop for IslandApp {
    fn drop(&mut self) {
        if let Some(e) = self.engine.take() {
            e.stop();
        }
    }
}
