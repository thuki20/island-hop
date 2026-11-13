#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! IslandHop — 屏幕顶部的一条「灵动岛」，把局域网键鼠 / 剪贴板共享与桌面小工具收进去。
//! 引擎复用 openhop-core，UI 用 egui 自绘，不依赖 WebView2 / Electron / Tauri。

mod features;
mod pages;
mod state;
mod windowing;

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, Frame, RichText, Stroke,
    StrokeKind, Vec2, ViewportBuilder, ViewportCommand,
};
use openhop_core::platform;
use openhop_core::{Config, Engine, Role, Status};
use pages::Action;
use state::*;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let initial = UiData::load(&state::data_path());
    let g = windowing::geometry(&initial.settings, false);
    let opts = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title("IslandHop")
            .with_inner_size([g.w, g.h])
            .with_min_inner_size([120.0, 24.0])
            .with_position([g.x, g.y])
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

/// egui 默认字体不含中文，运行时挂系统字体；失败则主要文案降级为英文。
fn setup_fonts(ctx: &egui::Context) -> bool {
    let paths: &[&str] = if cfg!(windows) {
        &["C:\\Windows\\Fonts\\msyh.ttc", "C:\\Windows\\Fonts\\msyhei.ttf", "C:\\Windows\\Fonts\\simhei.ttf"]
    } else if cfg!(target_os = "macos") {
        &["/System/Library/Fonts/PingFang.ttc", "/System/Library/Fonts/STHeiti Medium.ttc", "/System/Library/Fonts/Hiragino Sans GB.ttc"]
    } else {
        &[]
    };
    for p in paths {
        if let Ok(bytes) = fs::read(p) {
            let mut fonts = FontDefinitions::default();
            fonts.font_data.insert("cjk".to_owned(), Arc::new(FontData::from_owned(bytes)));
            fonts.families.entry(FontFamily::Proportional).or_default().insert(0, "cjk".to_owned());
            fonts.families.entry(FontFamily::Monospace).or_default().insert(0, "cjk".to_owned());
            ctx.set_fonts(fonts);
            return true;
        }
    }
    log::warn!("no CJK system font found; main labels fall back to English");
    false
}

fn config_path() -> PathBuf {
    let dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("island-hop");
    let _ = fs::create_dir_all(&dir);
    dir.join("config.toml")
}

fn t(cjk: bool, zh: &'static str, en: &'static str) -> &'static str {
    if cjk { zh } else { en }
}

struct IslandApp {
    cjk: bool,
    cfg_path: PathBuf,
    data_path: PathBuf,
    cfg: Config,
    data: UiData,
    engine: Option<Engine>,
    status: Option<Status>,
    last_poll: Instant,
    last_hover: Instant,
    last_save: Instant,
    last_geom: Option<windowing::Geometry>,
    expanded: bool,
    did_elevate: bool,
    pair_code: String,
    name_buf: String,
    pass_buf: String,
    permission_note: Option<String>,
    error: Option<String>,
    notice: Option<(String, Instant)>,
    quit: bool,
    clip: features::ClipboardHub,
    tasks: features::TasksHub,
    weather: features::WeatherHub,
    media: features::MediaHub,
}

impl IslandApp {
    fn new(cjk: bool) -> Self {
        let cfg_path = config_path();
        let data_path = state::data_path();
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
        let data = UiData::load(&data_path);
        let name_buf = cfg.name.clone();
        let pass_buf = cfg.passphrase.clone();
        let mut app = IslandApp {
            cjk,
            cfg_path,
            data_path,
            cfg,
            data,
            engine: None,
            status: None,
            last_poll: Instant::now() - Duration::from_secs(2),
            last_hover: Instant::now() - Duration::from_secs(2),
            last_save: Instant::now(),
            last_geom: None,
            expanded: false,
            did_elevate: false,
            pair_code: String::new(),
            name_buf,
            pass_buf,
            permission_note: None,
            error: None,
            notice: None,
            quit: false,
            clip: features::ClipboardHub::default(),
            tasks: features::TasksHub::default(),
            weather: features::WeatherHub::default(),
            media: features::MediaHub::default(),
        };
        app.restart();
        app
    }

    fn restart(&mut self) {
        if let Some(e) = self.engine.take() { e.stop(); }
        self.permission_note = platform::check_permissions(true);
        match Engine::start(self.cfg.clone(), Some(self.cfg_path.clone())) {
            Ok(e) => { self.engine = Some(e); self.error = None; }
            Err(err) => self.error = Some(err.to_string()),
        }
        self.last_poll = Instant::now() - Duration::from_secs(2);
    }

    fn apply(&mut self, action: Action) {
        match action {
            Action::Role(r) => { self.cfg.role = r; let _ = self.cfg.save(&self.cfg_path); self.restart(); }
            Action::SaveName => { self.cfg.name = self.name_buf.trim().to_string(); let _ = self.cfg.save(&self.cfg_path); self.restart(); }
            Action::SavePass => { self.cfg.passphrase = self.pass_buf.trim().to_string(); let _ = self.cfg.save(&self.cfg_path); self.restart(); }
            Action::Pair(device, code) => { if let Some(e) = &self.engine { e.pair(device, code); } }
            Action::Place(name, x) => {
                self.cfg.layout.place(&name, x, 0);
                let _ = self.cfg.save(&self.cfg_path);
                if let Some(e) = &self.engine { e.set_layout(self.cfg.layout.clone()); }
            }
            Action::Clipboard(on) => { self.cfg.clipboard_sync = on; let _ = self.cfg.save(&self.cfg_path); self.restart(); }
            Action::Restart => self.restart(),
            Action::Quit => self.quit = true,
            Action::OpenConfig => {
                if let Some(dir) = self.cfg_path.parent() { let _ = opener::open(dir); }
            }
            Action::OpenPrivacy(kind) => windowing::open_privacy_settings(&kind),
        }
    }

    fn dot(&self) -> (Color32, &'static str) {
        let c = self.cjk;
        match &self.status {
            Some(s) if !s.running || s.error.is_some() => (Color32::from_rgb(230, 90, 90), t(c, "已停止", "Stopped")),
            Some(s) if s.role == Role::Server && !s.active.is_empty() => (Color32::from_rgb(90, 205, 160), t(c, "跨屏中", "Remote")),
            Some(_) => (Color32::from_rgb(110, 200, 245), t(c, "已就绪", "Ready")),
            None => (Color32::from_gray(150), t(c, "启动中", "Starting")),
        }
    }

    fn collapsed(&mut self, ui: &mut egui::Ui, pal: Palette) {
        let (color, label) = self.dot();
        let minimal = self.data.settings.minimal;
        let clock = clock_str(self.data.settings.hour24);
        let extra: Vec<String> = if minimal { vec![] } else {
            [features::WeatherHub::summary(&self.data), self.media.summary(), features::TasksHub::summary(&self.data)]
                .into_iter().filter(|s| !s.is_empty()).collect()
        };
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 7.0;
            ui.label(RichText::new("●").size(10.0).color(color));
            ui.label(RichText::new(&self.cfg.name).size(12.5).strong().color(pal.fg));
            if !minimal { ui.label(RichText::new(label).size(10.5).color(pal.muted)); }
            for e in extra {
                ui.label(RichText::new(e).size(10.5).color(pal.accent));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(clock).size(11.0).color(pal.fg));
            });
        });
    }

    /// 跨屏控制页：原控制面板 + 连接诊断（Windows 未测时把「为什么没受控」写清楚）。
    fn kvm_ui(&mut self, ui: &mut egui::Ui, pal: Palette) -> Vec<Action> {
        let c = self.cjk;
        let mut acts: Vec<Action> = Vec::new();
        let mut pair_code = std::mem::take(&mut self.pair_code);
        let mut name_buf = std::mem::take(&mut self.name_buf);
        let mut pass_buf = std::mem::take(&mut self.pass_buf);

        {
            let s = self.status.as_ref();
            let (dot_color, state_label) = self.dot();
            let is_server = self.cfg.role == Role::Server;

            ui.horizontal(|ui| {
                ui.label(RichText::new("IslandHop").size(13.0).strong().color(pal.fg));
                ui.label(RichText::new("●").size(9.0).color(dot_color));
                ui.label(RichText::new(state_label).size(11.0).color(pal.muted));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(clock_str(self.data.settings.hour24)).size(11.0).color(pal.muted));
                });
            });
            ui.add_space(4.0);

            // ---- 连接诊断 ----
            ui.label(RichText::new(t(c, "连接诊断", "Connection")).size(12.0).strong().color(pal.accent));
            let role_line = if is_server {
                t(c, "本机 = 主控（共享本机键鼠）。对端必须设为「受控」才会被控制；本机不会变成受控。",
                     "This computer is the SERVER. The other side must be CLIENT.")
            } else {
                t(c, "本机 = 受控（被主控端键鼠控制）。需要主控端在线并配对。",
                     "This computer is the CLIENT; it needs an online SERVER.")
            };
            ui.label(RichText::new(role_line).size(10.5).color(pal.muted));
            if let Some(st) = s {
                let active = if st.active.is_empty() { t(c, "本机", "this Mac").to_string() } else { st.active.clone() };
                ui.label(RichText::new(format!(
                    "{} {} · {} {} · {} {} · {} {}",
                    t(c, "已发现", "discovered"), st.discovered.len(),
                    t(c, "已配对", "paired"), st.paired.len(),
                    t(c, "活动屏", "active"), active,
                    t(c, "端口", "port"), st.port,
                )).size(10.5).color(pal.muted));
                if st.needs_pairing {
                    ui.label(RichText::new(t(c,
                        "未配对：在下方输入主控端 6 位配对码，或两端填同一个共享口令。",
                        "Not paired: enter the 6-digit code from the server, or set the same passphrase."))
                        .size(10.5).color(Color32::from_rgb(240, 190, 110)));
                }
                if st.paired.is_empty() && st.discovered.is_empty() {
                    ui.label(RichText::new(t(c,
                        "对端未上线：Windows 端未安装 / 未运行 / 未放行 UDP 24851 时，这里会是空的。",
                        "No peer: the other computer is offline or UDP 24851 is blocked."))
                        .size(10.5).color(Color32::from_rgb(240, 190, 110)));
                }
            }
            let msg = s.map(|x| x.message.as_str()).unwrap_or("");
            ui.label(RichText::new(msg).size(10.5).color(pal.muted));
            if let Some(e) = &self.error {
                ui.label(RichText::new(e).size(10.5).color(Color32::from_rgb(240, 130, 120)));
            }
            if let Some(p) = &self.permission_note {
                ui.label(RichText::new(p).size(10.5).color(Color32::from_rgb(240, 190, 110)));
            }
            ui.horizontal(|ui| {
                if ui.small_button(t(c, "辅助功能授权", "Accessibility")).clicked() {
                    acts.push(Action::OpenPrivacy("accessibility".into()));
                }
                if ui.small_button(t(c, "输入监控授权", "Input monitoring")).clicked() {
                    acts.push(Action::OpenPrivacy("input".into()));
                }
                ui.label(RichText::new(t(c,
                    "授权后必须彻底退出重开才生效", "restart after granting")).size(10.0).color(pal.muted));
            });
            ui.add_space(5.0);
            ui.separator();
            ui.add_space(3.0);

            // ---- 控制面板 ----
            ui.horizontal(|ui| {
                ui.label(RichText::new(t(c, "角色", "Role")).size(11.0).color(pal.muted));
                if ui.selectable_label(is_server, t(c, "主控（共享键鼠）", "Server")).clicked() && !is_server {
                    acts.push(Action::Role(Role::Server));
                }
                if ui.selectable_label(!is_server, t(c, "受控", "Client")).clicked() && is_server {
                    acts.push(Action::Role(Role::Client));
                }
            });
            if is_server {
                if let Some(code) = s.and_then(|x| x.pairing_code.as_ref()) {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(t(c, "配对码", "Pair code")).size(11.0).color(pal.muted));
                        ui.label(RichText::new(code.as_str()).size(17.0).strong().color(Color32::from_rgb(120, 220, 180)));
                        ui.label(RichText::new(t(c, "在另一台输入", "enter on the other computer")).size(10.0).color(pal.muted));
                    });
                }
            } else {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(t(c, "配对码", "Pair code")).size(11.0).color(pal.muted));
                    ui.add(egui::TextEdit::singleline(&mut pair_code).desired_width(90.0).hint_text("000000"));
                });
                let servers: Vec<(String, String)> = s.map(|x| x.discovered.iter()
                    .filter(|d| d.role == Role::Server)
                    .map(|d| (d.device.clone(), d.name.clone())).collect()).unwrap_or_default();
                if servers.is_empty() {
                    ui.label(RichText::new(t(c, "尚未发现主控端", "No server discovered yet")).size(10.5).color(pal.muted));
                }
                for (device, name) in servers {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(name).size(11.0).color(pal.fg));
                        if ui.small_button(t(c, "配对", "Pair")).clicked() && !pair_code.trim().is_empty() {
                            acts.push(Action::Pair(device.clone(), pair_code.trim().to_string()));
                        }
                    });
                }
                if let Some(e) = s.and_then(|x| x.pair_error.as_ref()) {
                    ui.label(RichText::new(e).size(10.5).color(Color32::from_rgb(240, 130, 120)));
                }
            }
            ui.add_space(4.0);
            ui.label(RichText::new(t(c, "屏幕排列", "Screen layout")).size(11.0).color(pal.muted));
            let peers: Vec<(String, i32)> = s.map(|x| x.paired.iter().map(|p| {
                (p.name.clone(), x.layout.position(&p.name).map(|v| v.0).unwrap_or(0))
            }).collect()).unwrap_or_default();
            if peers.is_empty() {
                ui.label(RichText::new(t(c, "尚无已配对设备", "No paired computers")).size(10.5).color(pal.muted));
            }
            for (name, x) in peers {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("{}  (x={})", name, x)).size(10.5).color(pal.fg));
                    if ui.small_button(t(c, "设为左", "Left")).clicked() { acts.push(Action::Place(name.clone(), -1)); }
                    if ui.small_button(t(c, "设为右", "Right")).clicked() { acts.push(Action::Place(name.clone(), 1)); }
                });
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(t(c, "设备名", "Name")).size(11.0).color(pal.muted));
                ui.add(egui::TextEdit::singleline(&mut name_buf).desired_width(105.0));
                if ui.small_button(t(c, "保存", "Save")).clicked() { acts.push(Action::SaveName); }
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new(t(c, "共享口令", "Passphrase")).size(11.0).color(pal.muted));
                ui.add(egui::TextEdit::singleline(&mut pass_buf).password(true).desired_width(105.0));
                if ui.small_button(t(c, "保存", "Save")).clicked() { acts.push(Action::SavePass); }
            });
            let mut clip = self.cfg.clipboard_sync;
            if ui.checkbox(&mut clip, t(c, "剪贴板跨机同步", "Clipboard sync (engine)")).changed() {
                acts.push(Action::Clipboard(clip));
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.small_button(t(c, "重启引擎", "Restart")).clicked() { acts.push(Action::Restart); }
                if ui.small_button(t(c, "退出", "Quit")).clicked() { acts.push(Action::Quit); }
                if let Some(st) = s {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(format!("{} · :{}", st.os.label(), st.port)).size(10.0).color(pal.muted));
                    });
                }
            });
        }

        self.pair_code = pair_code;
        self.name_buf = name_buf;
        self.pass_buf = pass_buf;
        acts
    }

    fn expanded_ui(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let pal = self.data.settings.skin.palette();
        let c = self.cjk;
        let page = pages::tab_bar(ui, &mut self.data, pal, c);
        let app = &mut *self;
        let mut acts: Vec<Action> = Vec::new();
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            match page {
                PageId::Kvm => acts.extend(app.kvm_ui(ui, pal)),
                PageId::Music => app.media.ui(ui, pal, c),
                PageId::Weather => app.weather.ui(ui, &mut app.data, pal, c),
                PageId::Tasks => app.tasks.ui(ui, &mut app.data, pal, c),
                PageId::Shelf => features::shelf_ui(ui, &mut app.data, pal, c),
                PageId::Clipboard => app.clip.ui(ui, &mut app.data, pal, c),
                PageId::System => features::system_ui(ui, pal, c, app.permission_note.as_deref()),
                PageId::Quota => pages::quota_ui(ui, pal, c),
                PageId::Settings => acts.extend(pages::settings_ui(ui, &mut app.data, pal, c)),
            }
        });
        if let Some((msg, _)) = &self.notice {
            ui.add_space(3.0);
            ui.label(RichText::new(msg).size(10.5).color(pal.accent));
        }
        for a in acts { self.apply(a); }
        let _ = ctx;
    }
}

impl eframe::App for IslandApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_millis(150));
        if !self.did_elevate {
            self.did_elevate = windowing::elevate(frame);
        }
        if self.last_poll.elapsed() > Duration::from_millis(250) {
            if let Some(e) = &self.engine { self.status = Some(e.status()); }
            self.last_poll = Instant::now();
        }

        let pages = self.data.settings.pages.clone();
        let mut dirty = false;
        dirty |= self.clip.poll(&mut self.data, &self.cfg.passphrase);
        let (tick_dirty, notice) = features::tasks_tick(&mut self.data);
        dirty |= tick_dirty;
        if let Some(n) = notice { self.notice = Some((n, Instant::now())); }
        dirty |= self.weather.poll(&mut self.data, pages.contains(&PageId::Weather));
        self.media.poll(pages.contains(&PageId::Music));
        dirty |= features::shelf_poll_drops(ctx, &mut self.data);
        if let Some((_, at)) = &self.notice {
            if at.elapsed() > Duration::from_secs(8) { self.notice = None; }
        }
        if dirty || self.last_save.elapsed() > Duration::from_secs(3) {
            self.data.save(&self.data_path);
            self.last_save = Instant::now();
        }

        let hovering = ctx.input(|i| i.pointer.hover_pos().is_some());
        if hovering { self.last_hover = Instant::now(); }
        self.expanded = if self.expanded {
            self.last_hover.elapsed() < Duration::from_millis(900)
        } else { hovering };

        let g = windowing::geometry(&self.data.settings, self.expanded);
        let changed = self.last_geom.map(|p| {
            (p.x - g.x).abs() > 0.5 || (p.y - g.y).abs() > 0.5
                || (p.w - g.w).abs() > 0.5 || (p.h - g.h).abs() > 0.5
        }).unwrap_or(true);
        if changed {
            ctx.send_viewport_cmd(ViewportCommand::InnerSize(Vec2::new(g.w, g.h)));
            ctx.send_viewport_cmd(ViewportCommand::OuterPosition(egui::pos2(g.x, g.y)));
            log::info!("island geometry: x={:.0} y={:.0} w={:.0} h={:.0} expanded={}", g.x, g.y, g.w, g.h, self.expanded);
            self.last_geom = Some(g);
        }
        if self.quit {
            if let Some(e) = self.engine.take() { e.stop(); }
            self.data.save(&self.data_path);
            ctx.send_viewport_cmd(ViewportCommand::Close);
            return;
        }

        let expanded = self.expanded;
        let pal = self.data.settings.skin.palette();
        egui::CentralPanel::default()
            .frame(Frame::default().fill(Color32::TRANSPARENT))
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                let radius = if expanded { 15 } else if self.data.settings.shape == Shape::Notch { 16 } else { 20 };
                ui.painter().rect_filled(rect, CornerRadius::same(radius), pal.bg);
                ui.painter().rect_stroke(rect, CornerRadius::same(radius),
                    Stroke::new(0.6_f32, pal.border), StrokeKind::Outside);
                let inner = rect.shrink2(Vec2::new(12.0, if expanded { 9.0 } else { 6.0 }));
                let app = &mut *self;
                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(inner), |ui| {
                    if expanded { app.expanded_ui(ctx, ui); } else { app.collapsed(ui, pal); }
                });
            });
    }
}

impl Drop for IslandApp {
    fn drop(&mut self) {
        if let Some(e) = self.engine.take() { e.stop(); }
    }
}
