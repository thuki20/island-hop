//! 岛仔主力功能里**跨平台、无需外部凭据**的部分。
//! 明确边界：不假装有 AI 额度 / 邮件 / 微信 / 远程日历 / 歌词（见 quota_ui）。

use crate::state::*;
use eframe::egui::{self, Color32, RichText};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

pub fn t(cjk: bool, zh: &'static str, en: &'static str) -> &'static str {
    if cjk { zh } else { en }
}

fn hint(ui: &mut egui::Ui, text: impl Into<String>, pal: Palette) {
    ui.label(RichText::new(text).size(11.0).color(pal.muted));
}

fn title(ui: &mut egui::Ui, text: &'static str, pal: Palette) {
    ui.label(RichText::new(text).size(12.5).strong().color(pal.fg));
    ui.add_space(3.0);
}

fn shorten(s: &str, n: usize) -> String {
    let one: String = s.chars().filter(|c| !c.is_control()).take(n).collect();
    if s.chars().count() > n { format!("{one}…") } else { one }
}

fn curl(url: &str) -> Result<String, String> {
    let out = std::process::Command::new("curl")
        .args(["-sS", "--max-time", "8", url])
        .output()
        .map_err(|e| format!("curl 启动失败: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

// ------------------------------------------------------------------ clipboard

pub struct ClipboardHub {
    board: Option<arboard::Clipboard>,
    last_seen: String,
    last_written: String,
    last_poll: Instant,
    pub note: Option<String>,
}

impl Default for ClipboardHub {
    fn default() -> Self {
        ClipboardHub {
            board: None,
            last_seen: String::new(),
            last_written: String::new(),
            last_poll: Instant::now() - Duration::from_secs(2),
            note: None,
        }
    }
}

fn looks_secret(s: &str, guard: &str) -> bool {
    if !guard.is_empty() && s == guard { return true; }
    let t = s.trim();
    ["sk-", "ghp_", "github_pat_", "-----BEGIN", "Bearer ", "xoxb-", "AKIA"]
        .iter().any(|p| t.starts_with(p))
        || t.contains("PRIVATE KEY")
        || t.len() > 2000
}

impl ClipboardHub {
    /// 轮询系统剪贴板，新增一条历史。返回是否改动了数据。
    pub fn poll(&mut self, data: &mut UiData, guard: &str) -> bool {
        if !data.settings.clipboard_enabled || self.last_poll.elapsed() < Duration::from_millis(800) {
            return false;
        }
        self.last_poll = Instant::now();
        if self.board.is_none() {
            self.board = arboard::Clipboard::new().ok();
            if self.board.is_none() {
                self.note = Some("无法访问系统剪贴板".into());
                return false;
            }
        }
        let text = {
            let Some(b) = self.board.as_mut() else { return false };
            match b.get_text() { Ok(v) => v, Err(_) => return false }
        };
        if text.is_empty() || text == self.last_seen {
            return false;
        }
        self.last_seen = text.clone();
        // 跳过「本端点击复制」回写产生的回声；随后清掉标记，
        // 这样之后从别的应用再次复制同样内容仍能进历史。
        let echo = !self.last_written.is_empty() && text == self.last_written;
        self.last_written.clear();
        if echo || looks_secret(&text, guard) {
            return false;
        }
        self.note = None;
        if data.clip.iter().any(|e| e.text == text) {
            data.clip.retain(|e| e.text != text);
        }
        data.clip.push(ClipEntry { text, at: now(), pinned: false });
        let mut keep: Vec<ClipEntry> = data.clip.iter().filter(|e| e.pinned).cloned().collect();
        let mut rest: Vec<ClipEntry> = data.clip.iter().filter(|e| !e.pinned).cloned().collect();
        rest.reverse();
        rest.truncate(30_usize.saturating_sub(keep.len()));
        rest.reverse();
        keep.extend(rest);
        data.clip = keep;
        true
    }

    pub fn copy(&mut self, text: &str) {
        if self.board.is_none() { self.board = arboard::Clipboard::new().ok(); }
        if let Some(b) = self.board.as_mut() {
            if b.set_text(text.to_string()).is_ok() {
                self.last_written = text.to_string();
                self.last_seen = text.to_string();
            }
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, data: &mut UiData, pal: Palette, cjk: bool) {
        title(ui, t(cjk, "最近复制的文字（最多 30 条）", "Recent text (max 30)"), pal);
        hint(ui, t(cjk,
            "点「复制」写回系统剪贴板；疑似密钥/口令不会记录；本机会绕过本端重复写入。",
            "Click copy to write back. Secret-looking text is skipped."), pal);
        if let Some(n) = &self.note {
            ui.label(RichText::new(n).size(11.0).color(Color32::from_rgb(240, 170, 110)));
        }
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let mut on = data.settings.clipboard_enabled;
            if ui.checkbox(&mut on, t(cjk, "记录剪贴板历史", "Record history")).changed() {
                data.settings.clipboard_enabled = on;
            }
            if ui.small_button(t(cjk, "清空", "Clear")).clicked() {
                data.clip.retain(|e| e.pinned);
            }
        });
        ui.add_space(4.0);
        let mut to_copy: Option<String> = None;
        let mut to_delete: Option<usize> = None;
        egui::ScrollArea::vertical().max_height(250.0).show(ui, |ui| {
            let entries: Vec<(usize, ClipEntry)> = data.clip.iter().cloned().enumerate().collect();
            for (idx, e) in entries.iter().rev() {
                ui.horizontal(|ui| {
                    if ui.small_button(t(cjk, "复制", "Copy")).clicked() { to_copy = Some(e.text.clone()); }
                    let pin = if e.pinned { "★" } else { "☆" };
                    if ui.small_button(pin).clicked() {
                        if let Some(x) = data.clip.get_mut(*idx) { x.pinned = !x.pinned; }
                    }
                    if ui.small_button("×").clicked() { to_delete = Some(*idx); }
                    ui.label(RichText::new(format!("{}  {}", time_str(e.at), shorten(&e.text, 40)))
                        .size(11.0).color(pal.fg));
                });
            }
            if data.clip.is_empty() {
                hint(ui, t(cjk, "还没有记录。复制一段文字试试。", "Nothing yet. Copy some text."), pal);
            }
        });
        if let Some(text) = to_copy { self.copy(&text); }
        if let Some(i) = to_delete { if i < data.clip.len() { data.clip.remove(i); } }
    }
}

// ------------------------------------------------------------------ tasks

pub struct TasksHub {
    pub todo_buf: String,
    pub repeat: Repeat,
}

impl Default for TasksHub {
    fn default() -> Self { TasksHub { todo_buf: String::new(), repeat: Repeat::Once } }
}

/// 每天/每周重复待办到期复位；番茄钟走时；饮水跨天清零。
pub fn tasks_tick(data: &mut UiData) -> (bool, Option<String>) {
    let mut changed = false;
    let n = now();
    if data.water.day != today() {
        data.water.day = today();
        data.water.total = 0;
        changed = true;
    }
    for todo in data.todos.iter_mut() {
        if todo.repeat != Repeat::Once && todo.done && todo.due > 0 && n >= todo.due {
            todo.done = false;
            todo.last_done = n;
            changed = true;
        }
    }
    let mut notice = None;
    if data.pomo.running {
        let left = (data.pomo.ends_at - n).max(0);
        if left != data.pomo.left { data.pomo.left = left; changed = true; }
        if left == 0 {
            data.pomo.running = false;
            let was_focus = data.pomo.phase == Phase::Focus;
            if was_focus { data.pomo.sessions += 1; }
            data.pomo.phase = if was_focus { Phase::Break } else { Phase::Focus };
            let mins = if was_focus { data.settings.break_min } else { data.settings.focus_min };
            data.pomo.left = mins as i64 * 60;
            notice = Some(if was_focus { "专注结束，休息一下".into() } else { "休息结束，开始专注".into() });
            changed = true;
        }
    }
    (changed, notice)
}

fn mmss(sec: i64) -> String {
    format!("{:02}:{:02}", sec / 60, sec % 60)
}

impl TasksHub {
    pub fn ui(&mut self, ui: &mut egui::Ui, data: &mut UiData, pal: Palette, cjk: bool) {
        title(ui, t(cjk, "待办", "To-do"), pal);
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.todo_buf).desired_width(210.0)
                .hint_text(t(cjk, "写一条待办", "New task")));
            egui::ComboBox::from_id_salt("repeat").width(70.0)
                .show_ui(ui, |ui| {
                    for r in [Repeat::Once, Repeat::Daily, Repeat::Weekly] {
                        ui.selectable_value(&mut self.repeat, r, r.label());
                    }
                });
            let add = ui.small_button(t(cjk, "添加", "Add")).clicked();
            if (add || ui.input(|i| i.key_pressed(egui::Key::Enter))) && !self.todo_buf.trim().is_empty() {
                let step = match self.repeat { Repeat::Once => 0, Repeat::Daily => 86_400, Repeat::Weekly => 604_800 };
                data.todos.push(Todo {
                    title: self.todo_buf.trim().to_string(), done: false, repeat: self.repeat,
                    due: if step > 0 { now() + step } else { 0 }, last_done: 0,
                });
                self.todo_buf.clear();
            }
        });
        ui.add_space(3.0);
        let mut remove: Option<usize> = None;
        let items: Vec<Todo> = data.todos.clone();
        for (i, todo) in items.iter().enumerate() {
            ui.horizontal(|ui| {
                let mut done = todo.done;
                if ui.checkbox(&mut done, "").changed() {
                    if let Some(x) = data.todos.get_mut(i) { x.done = done; x.last_done = now(); }
                }
                let txt = format!("{} · {}", shorten(&todo.title, 26), todo.repeat.label());
                let mut rt = RichText::new(txt).size(11.0).color(if todo.done { pal.muted } else { pal.fg });
                if todo.done { rt = rt.strikethrough(); }
                ui.label(rt);
                if ui.small_button("×").clicked() { remove = Some(i); }
            });
        }
        if let Some(i) = remove { if i < data.todos.len() { data.todos.remove(i); } }

        ui.add_space(8.0);
        ui.separator();
        title(ui, t(cjk, "专注 / 番茄钟", "Focus timer"), pal);
        ui.horizontal(|ui| {
            let phase = if data.pomo.phase == Phase::Focus {
                t(cjk, "专注", "Focus")
            } else { t(cjk, "休息", "Break") };
            ui.label(RichText::new(format!("{}  {}", phase, mmss(data.pomo.left)))
                .size(18.0).strong().color(pal.accent));
            ui.label(RichText::new(format!("×{}", data.pomo.sessions)).size(11.0).color(pal.muted));
            if ui.small_button(if data.pomo.running { t(cjk, "暂停", "Pause") } else { t(cjk, "开始", "Start") }).clicked() {
                if data.pomo.running {
                    data.pomo.running = false;
                } else {
                    let mins = if data.pomo.phase == Phase::Focus { data.settings.focus_min } else { data.settings.break_min };
                    if data.pomo.left <= 0 { data.pomo.left = mins as i64 * 60; }
                    data.pomo.ends_at = now() + data.pomo.left;
                    data.pomo.running = true;
                }
            }
            if ui.small_button(t(cjk, "重置", "Reset")).clicked() {
                let mins = if data.pomo.phase == Phase::Focus { data.settings.focus_min } else { data.settings.break_min };
                data.pomo.running = false;
                data.pomo.left = mins as i64 * 60;
            }
        });

        ui.add_space(8.0);
        ui.separator();
        title(ui, t(cjk, "饮水", "Water"), pal);
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("{} / {} ml", data.water.total, data.settings.water_goal))
                .size(13.0).color(pal.fg));
            if ui.small_button(format!("+{} ml", data.settings.water_cup)).clicked() {
                data.water.total += data.settings.water_cup;
                data.water.last = now();
            }
            if ui.small_button(t(cjk, "撤销一杯", "Undo")).clicked() {
                data.water.total = data.water.total.saturating_sub(data.settings.water_cup);
            }
        });
        hint(ui, t(cjk, "提醒只在岛内显示；退出/睡眠期间不会提醒（与岛仔一致）。",
            "Reminders are in-island only; no alerts while asleep."), pal);
    }

    pub fn summary(data: &UiData) -> String {
        let n = data.todos.iter().filter(|x| !x.done).count();
        if n == 0 { String::new() } else { format!("☑ {n}") }
    }
}

// ------------------------------------------------------------------ shelf

pub fn shelf_poll_drops(ctx: &egui::Context, data: &mut UiData) -> bool {
    let paths: Vec<PathBuf> = ctx.input(|i| {
        i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect()
    });
    let mut changed = false;
    for p in paths {
        let s = p.to_string_lossy().to_string();
        if !data.shelf.contains(&s) {
            data.shelf.push(s);
            changed = true;
        }
    }
    if data.shelf.len() > 50 { data.shelf.drain(0..data.shelf.len() - 50); }
    changed
}

pub fn shelf_ui(ui: &mut egui::Ui, data: &mut UiData, pal: Palette, cjk: bool) {
    title(ui, t(cjk, "文件置物架", "File shelf"), pal);
    hint(ui, t(cjk, "把文件拖到岛上临时收集；移除只删记录，不删原文件。",
        "Drop files onto the island. Removing a row never deletes the file."), pal);
    ui.add_space(4.0);
    let mut remove: Option<usize> = None;
    egui::ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
        let items = data.shelf.clone();
        for (i, s) in items.iter().enumerate() {
            let name = PathBuf::from(s).file_name().map(|x| x.to_string_lossy().to_string())
                .unwrap_or_else(|| s.clone());
            ui.horizontal(|ui| {
                if ui.small_button(t(cjk, "打开", "Open")).clicked() { let _ = opener::open(s); }
                if ui.small_button(t(cjk, "定位", "Reveal")).clicked() { let _ = opener::reveal(s); }
                if ui.small_button("×").clicked() { remove = Some(i); }
                ui.label(RichText::new(shorten(&name, 34)).size(11.0).color(pal.fg));
            });
        }
        if data.shelf.is_empty() {
            hint(ui, t(cjk, "还没有文件。", "Empty."), pal);
        }
    });
    if let Some(i) = remove { if i < data.shelf.len() { data.shelf.remove(i); } }
}

// ------------------------------------------------------------------ weather

pub struct WeatherHub {
    rx: Option<Receiver<Result<WeatherCache, String>>>,
    in_flight: bool,
    last_fetch: Instant,
    pub status: String,
}

impl Default for WeatherHub {
    fn default() -> Self {
        WeatherHub { rx: None, in_flight: false, last_fetch: Instant::now() - Duration::from_secs(3600), status: String::new() }
    }
}

fn wmo(code: i32) -> &'static str {
    match code {
        0 => "晴", 1 => "大致晴朗", 2 => "局部多云", 3 => "阴",
        45 | 48 => "雾", 51 | 53 | 55 | 56 | 57 => "毛毛雨",
        61 | 63 | 65 | 66 | 67 => "雨", 71 | 73 | 75 | 77 => "雪",
        80 | 81 | 82 => "阵雨", 85 | 86 => "阵雪", 95 | 96 | 99 => "雷雨",
        _ => "未知",
    }
}

impl WeatherHub {
    pub fn refresh(&mut self, data: &UiData) {
        if self.in_flight { return; }
        let city = data.settings.city.clone();
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        self.in_flight = true;
        self.last_fetch = Instant::now();
        self.status = "正在获取天气…".into();
        std::thread::spawn(move || {
            let result = (|| -> Result<WeatherCache, String> {
                let (lat, lon, place) = if city.trim().is_empty() {
                    let raw = curl("https://ipwho.is/")?;
                    let v: serde_json::Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
                    let lat = v["latitude"].as_f64().ok_or("定位失败")?;
                    let lon = v["longitude"].as_f64().ok_or("定位失败")?;
                    let place = v["city"].as_str().unwrap_or("当前位置").to_string();
                    (lat, lon, place)
                } else {
                    let q = city.trim().replace(' ', "%20");
                    let raw = curl(&format!("https://geocoding-api.open-meteo.com/v1/search?name={q}&count=1&language=zh"))?;
                    let v: serde_json::Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
                    let first = v["results"].get(0).ok_or("找不到这个城市")?;
                    let lat = first["latitude"].as_f64().ok_or("城市纬度缺失")?;
                    let lon = first["longitude"].as_f64().ok_or("城市经度缺失")?;
                    let place = first["name"].as_str().unwrap_or(city.trim()).to_string();
                    (lat, lon, place)
                };
                let url = format!(
                    "https://api.open-meteo.com/v1/forecast?latitude={lat}&longitude={lon}&current=temperature_2m,relative_humidity_2m,apparent_temperature,weather_code&daily=weather_code,temperature_2m_max,temperature_2m_min,precipitation_probability_max&timezone=auto&forecast_days=5");
                let raw = curl(&url)?;
                let v: serde_json::Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
                let cur = &v["current"];
                let num = |x: &serde_json::Value| x.as_f64().unwrap_or(0.0) as f32;
                let mut daily = Vec::new();
                if let Some(arr) = v["daily"]["time"].as_array() {
                    for (i, d) in arr.iter().enumerate() {
                        daily.push(Daily {
                            date: d.as_str().unwrap_or("").to_string(),
                            hi: num(&v["daily"]["temperature_2m_max"][i]),
                            lo: num(&v["daily"]["temperature_2m_min"][i]),
                            code: v["daily"]["weather_code"][i].as_i64().unwrap_or(0) as i32,
                            rain: v["daily"]["precipitation_probability_max"][i].as_i64().unwrap_or(0) as i32,
                        });
                    }
                }
                Ok(WeatherCache {
                    place,
                    temp: num(&cur["temperature_2m"]),
                    apparent: num(&cur["apparent_temperature"]),
                    humidity: cur["relative_humidity_2m"].as_i64().unwrap_or(0) as i32,
                    code: cur["weather_code"].as_i64().unwrap_or(0) as i32,
                    updated_at: now(),
                    daily,
                })
            })();
            let _ = tx.send(result);
        });
    }

    pub fn poll(&mut self, data: &mut UiData, enabled: bool) -> bool {
        if let Some(rx) = &self.rx {
            if let Ok(r) = rx.try_recv() {
                self.rx = None;
                self.in_flight = false;
                match r {
                    Ok(w) => { data.weather = Some(w); self.status.clear(); }
                    Err(e) => self.status = format!("获取失败：{e}"),
                }
                return true;
            }
        }
        if enabled && self.last_fetch.elapsed() > Duration::from_secs(900) && !self.in_flight {
            self.refresh(data);
        }
        false
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, data: &mut UiData, pal: Palette, cjk: bool) {
        title(ui, t(cjk, "天气 · Open-Meteo（无需密钥）", "Weather · Open-Meteo"), pal);
        ui.horizontal(|ui| {
            ui.label(RichText::new(t(cjk, "城市", "City")).size(11.0).color(pal.muted));
            ui.add(egui::TextEdit::singleline(&mut data.settings.city).desired_width(150.0)
                .hint_text(t(cjk, "留空=按网络出口自动定位", "blank = auto locate")));
            if ui.small_button(t(cjk, "刷新", "Refresh")).clicked() { self.refresh(data); }
        });
        if !self.status.is_empty() {
            ui.label(RichText::new(&self.status).size(11.0).color(Color32::from_rgb(240, 180, 110)));
        }
        ui.add_space(4.0);
        if let Some(w) = &data.weather {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{:.0}°", w.temp)).size(30.0).strong().color(pal.fg));
                ui.label(RichText::new(format!("{}  {}", w.place, wmo(w.code))).size(13.0).color(pal.accent));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(time_str(w.updated_at)).size(10.0).color(pal.muted));
                });
            });
            hint(ui, format!("体感 {:.0}°  ·  湿度 {}%", w.apparent, w.humidity), pal);
            ui.add_space(5.0);
            for d in &w.daily {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&d.date).size(11.0).color(pal.muted));
                    ui.label(RichText::new(wmo(d.code)).size(11.0).color(pal.fg));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(format!("{:.0}° / {:.0}°  ·  {}%", d.hi, d.lo, d.rain))
                            .size(11.0).color(pal.fg));
                    });
                });
            }
        } else {
            hint(ui, t(cjk, "还没有天气数据。点「刷新」获取。", "No data yet. Press refresh."), pal);
        }
        hint(ui, t(cjk, "需要本机 curl 可联网；结果缓存在本机，断网显示上次数据。",
            "Requires curl + network; last result is cached locally."), pal);
    }

    pub fn summary(data: &UiData) -> String {
        data.weather.as_ref().map(|w| format!("{} {:.0}°", wmo(w.code), w.temp)).unwrap_or_default()
    }
}

// ------------------------------------------------------------------ media

pub struct MediaHub {
    media: openhop_core::extras::media::Media,
    last_poll: Instant,
    pub current: Option<openhop_core::extras::media::NowPlaying>,
}

impl Default for MediaHub {
    fn default() -> Self {
        MediaHub { media: openhop_core::extras::media::Media::new(), last_poll: Instant::now() - Duration::from_secs(3), current: None }
    }
}

impl MediaHub {
    pub fn poll(&mut self, enabled: bool) {
        if !enabled || self.last_poll.elapsed() < Duration::from_secs(2) { return; }
        self.last_poll = Instant::now();
        self.current = self.media.now();
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, pal: Palette, cjk: bool) {
        use openhop_core::extras::media::MediaCmd;
        title(ui, t(cjk, "正在播放", "Now playing"), pal);
        if let Some(m) = &self.current {
            ui.label(RichText::new(shorten(&m.title, 30)).size(16.0).strong().color(pal.fg));
            ui.label(RichText::new(format!("{}  ·  {}", shorten(&m.artist, 24), m.app)).size(11.0).color(pal.muted));
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                if ui.small_button("⏮").clicked() { self.media.command(MediaCmd::Previous); }
                if ui.small_button(if m.playing { "⏸" } else { "▶" }).clicked() { self.media.command(MediaCmd::PlayPause); }
                if ui.small_button("⏭").clicked() { self.media.command(MediaCmd::Next); }
                if let (Some(p), Some(d)) = (m.position, m.duration) {
                    ui.label(RichText::new(format!("{:02}:{:02} / {:02}:{:02}",
                        (p / 60.0) as i32, (p % 60.0) as i32, (d / 60.0) as i32, (d % 60.0) as i32))
                        .size(11.0).color(pal.muted));
                }
            });
        } else {
            hint(ui, t(cjk, "没有读到播放信息。", "Nothing playing."), pal);
        }
        ui.add_space(6.0);
        hint(ui, t(cjk,
            "macOS 只支持 Spotify / 音乐（AppleScript，系统不给第三方媒体信息）；Windows 走系统媒体会话。歌词未实现。",
            "macOS: Spotify/Music only. Windows: system media session. Lyrics not implemented."), pal);
    }

    pub fn summary(&self) -> String {
        self.current.as_ref().filter(|m| m.playing)
            .map(|m| format!("♪ {}", shorten(&m.title, 12))).unwrap_or_default()
    }
}

// ------------------------------------------------------------------ system

fn local_ip() -> String {
    std::net::UdpSocket::bind("0.0.0.0:0")
        .and_then(|s| { let _ = s.connect("8.8.8.8:80"); s.local_addr() })
        .map(|a| a.ip().to_string()).unwrap_or_else(|_| "—".into())
}

pub fn system_ui(ui: &mut egui::Ui, pal: Palette, cjk: bool, permission: Option<&str>) {
    use openhop_core::extras::system as sys;
    title(ui, t(cjk, "本机状态", "This computer"), pal);
    let row = |ui: &mut egui::Ui, k: String, v: String| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(k).size(11.0).color(pal.muted));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(v).size(11.0).color(pal.fg));
            });
        });
    };
    row(ui, t(cjk, "本机 IP", "Local IP").into(), local_ip());
    row(ui, t(cjk, "键鼠端口", "KVM ports").into(), "TCP 24850 / UDP 24851".into());
    if let Some((pct, charging)) = sys::battery() {
        row(ui, t(cjk, "电量", "Battery").into(), format!("{}%{}", pct, if charging { " ⚡" } else { "" }));
    }
    if let Some((free, total)) = sys::disk() {
        row(ui, t(cjk, "磁盘可用", "Disk free").into(),
            format!("{:.1} / {:.1} GB", free as f64 / 1e9, total as f64 / 1e9));
    }
    if let Some(dark) = sys::dark_mode() {
        row(ui, t(cjk, "深色模式", "Dark mode").into(), if dark { "开".into() } else { "关".into() });
    }
    if let Some(dnd) = sys::dnd() {
        row(ui, t(cjk, "勿扰", "Do Not Disturb").into(), if dnd { "开".into() } else { "关".into() });
    }
    if let Some(secs) = sys::idle_secs() {
        row(ui, t(cjk, "空闲", "Idle").into(), format!("{secs} s"));
    }
    ui.add_space(5.0);
    if let Some(p) = permission {
        ui.label(RichText::new(p).size(11.0).color(Color32::from_rgb(240, 190, 110)));
    }
    hint(ui, t(cjk, "音量 / 大写锁定 / 输入语言 / 蓝牙状态未实现（跨平台接口不统一）。",
        "Volume / CapsLock / input language / Bluetooth not implemented."), pal);
}
