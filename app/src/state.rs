//! 界面设置与本机数据（皮肤、页面、剪贴板历史、待办、置物架、天气缓存）。
//! 全部保存在 island-hop/ui.json，不联网、不上传。

use eframe::egui::Color32;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub fn data_path() -> PathBuf {
    let dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("island-hop");
    let _ = fs::create_dir_all(&dir);
    dir.join("ui.json")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Skin {
    Classic,
    Cream,
    GlassLight,
    GlassDark,
    Cloud,
}

impl Skin {
    pub fn label(self) -> &'static str {
        match self {
            Skin::Classic => "经典黑",
            Skin::Cream => "奶油",
            Skin::GlassLight => "浅色",
            Skin::GlassDark => "深色",
            Skin::Cloud => "悬浮云",
        }
    }
    pub fn all() -> [Skin; 5] {
        [Skin::Classic, Skin::Cream, Skin::GlassLight, Skin::GlassDark, Skin::Cloud]
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub bg: Color32,
    pub fg: Color32,
    pub muted: Color32,
    pub accent: Color32,
    pub border: Color32,
    /// 备用卡片底色：下一版给各功能页加卡片容器时用。
    #[allow(dead_code)]
    pub card: Color32,
}

impl Skin {
    pub fn palette(self) -> Palette {
        match self {
            Skin::Classic => Palette {
                bg: Color32::from_rgba_unmultiplied(22, 23, 27, 244),
                fg: Color32::WHITE,
                muted: Color32::from_gray(168),
                accent: Color32::from_rgb(110, 210, 255),
                border: Color32::from_rgba_unmultiplied(255, 255, 255, 32),
                card: Color32::from_rgba_unmultiplied(255, 255, 255, 14),
            },
            Skin::Cream => Palette {
                bg: Color32::from_rgba_unmultiplied(250, 245, 234, 246),
                fg: Color32::from_gray(38),
                muted: Color32::from_gray(105),
                accent: Color32::from_rgb(212, 118, 62),
                border: Color32::from_rgba_unmultiplied(90, 70, 50, 55),
                card: Color32::from_rgba_unmultiplied(120, 95, 65, 18),
            },
            Skin::GlassLight => Palette {
                bg: Color32::from_rgba_unmultiplied(238, 242, 248, 225),
                fg: Color32::from_gray(30),
                muted: Color32::from_gray(100),
                accent: Color32::from_rgb(48, 126, 220),
                border: Color32::from_rgba_unmultiplied(255, 255, 255, 120),
                card: Color32::from_rgba_unmultiplied(255, 255, 255, 80),
            },
            Skin::GlassDark => Palette {
                bg: Color32::from_rgba_unmultiplied(30, 34, 44, 214),
                fg: Color32::from_gray(242),
                muted: Color32::from_gray(170),
                accent: Color32::from_rgb(130, 190, 255),
                border: Color32::from_rgba_unmultiplied(255, 255, 255, 45),
                card: Color32::from_rgba_unmultiplied(255, 255, 255, 18),
            },
            Skin::Cloud => Palette {
                bg: Color32::from_rgba_unmultiplied(58, 48, 84, 232),
                fg: Color32::from_gray(248),
                muted: Color32::from_gray(190),
                accent: Color32::from_rgb(255, 176, 210),
                border: Color32::from_rgba_unmultiplied(255, 255, 255, 60),
                card: Color32::from_rgba_unmultiplied(255, 255, 255, 20),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Dock {
    Top,
    Left,
    Right,
}

impl Dock {
    pub fn label(self) -> &'static str {
        match self { Dock::Top => "顶部", Dock::Left => "左侧", Dock::Right => "右侧" }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Shape {
    Notch,
    Capsule,
}

impl Shape {
    pub fn label(self) -> &'static str {
        match self { Shape::Notch => "刘海", Shape::Capsule => "胶囊" }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PageId {
    Kvm,
    Music,
    Weather,
    Tasks,
    Shelf,
    Clipboard,
    System,
    Quota,
    Settings,
}

impl PageId {
    pub fn label(self, cjk: bool) -> &'static str {
        match (cjk, self) {
            (true, PageId::Kvm) => "跨屏",
            (true, PageId::Music) => "音乐",
            (true, PageId::Weather) => "天气",
            (true, PageId::Tasks) => "待办",
            (true, PageId::Shelf) => "置物",
            (true, PageId::Clipboard) => "剪贴板",
            (true, PageId::System) => "系统",
            (true, PageId::Quota) => "额度",
            (true, PageId::Settings) => "设置",
            (false, PageId::Kvm) => "KVM",
            (false, PageId::Music) => "Music",
            (false, PageId::Weather) => "Weather",
            (false, PageId::Tasks) => "Tasks",
            (false, PageId::Shelf) => "Shelf",
            (false, PageId::Clipboard) => "Clip",
            (false, PageId::System) => "System",
            (false, PageId::Quota) => "Quota",
            (false, PageId::Settings) => "Setup",
        }
    }
    pub fn all() -> [PageId; 9] {
        [PageId::Kvm, PageId::Music, PageId::Weather, PageId::Tasks, PageId::Shelf,
         PageId::Clipboard, PageId::System, PageId::Quota, PageId::Settings]
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub skin: Skin,
    pub dock: Dock,
    pub shape: Shape,
    pub scale: f32,
    pub hour24: bool,
    pub minimal: bool,
    pub pages: Vec<PageId>,
    pub selected: PageId,
    pub city: String,
    pub clipboard_enabled: bool,
    pub focus_min: u32,
    pub break_min: u32,
    pub water_goal: u32,
    pub water_cup: u32,
    pub water_interval: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            skin: Skin::Classic,
            dock: Dock::Top,
            shape: Shape::Notch,
            scale: 1.0,
            hour24: true,
            minimal: false,
            pages: PageId::all().to_vec(),
            selected: PageId::Kvm,
            city: String::new(),
            clipboard_enabled: true,
            focus_min: 25,
            break_min: 5,
            water_goal: 1500,
            water_cup: 250,
            water_interval: 60,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClipEntry {
    pub text: String,
    pub at: i64,
    pub pinned: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Repeat {
    Once,
    Daily,
    Weekly,
}

impl Repeat {
    pub fn label(self) -> &'static str {
        match self { Repeat::Once => "单次", Repeat::Daily => "每天", Repeat::Weekly => "每周" }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Todo {
    pub title: String,
    pub done: bool,
    pub repeat: Repeat,
    pub due: i64,
    pub last_done: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Focus,
    Break,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Pomo {
    pub running: bool,
    pub phase: Phase,
    pub ends_at: i64,
    pub left: i64,
    pub sessions: u32,
}

impl Default for Pomo {
    fn default() -> Self { Pomo { running: false, phase: Phase::Focus, ends_at: 0, left: 25 * 60, sessions: 0 } }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Water {
    pub day: String,
    pub total: u32,
    pub last: i64,
}

impl Default for Water {
    fn default() -> Self { Water { day: String::new(), total: 0, last: 0 } }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Daily {
    pub date: String,
    pub hi: f32,
    pub lo: f32,
    pub code: i32,
    pub rain: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WeatherCache {
    pub place: String,
    pub temp: f32,
    pub apparent: f32,
    pub humidity: i32,
    pub code: i32,
    pub updated_at: i64,
    pub daily: Vec<Daily>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct UiData {
    pub settings: Settings,
    pub clip: Vec<ClipEntry>,
    pub todos: Vec<Todo>,
    pub pomo: Pomo,
    pub water: Water,
    pub shelf: Vec<String>,
    pub weather: Option<WeatherCache>,
}

impl UiData {
    pub fn load(path: &Path) -> Self {
        match fs::read_to_string(path) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_else(|e| {
                log::warn!("ui.json parse failed, using defaults: {e}");
                UiData::default()
            }),
            Err(_) => UiData::default(),
        }
    }

    pub fn save(&self, path: &Path) {
        match serde_json::to_string_pretty(self) {
            Ok(s) => {
                let tmp = path.with_extension("json.tmp");
                if fs::write(&tmp, s).is_ok() {
                    let _ = fs::rename(tmp, path);
                }
            }
            Err(e) => log::warn!("ui.json serialize failed: {e}"),
        }
    }
}

pub fn now() -> i64 {
    chrono::Local::now().timestamp()
}

pub fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

pub fn clock_str(hour24: bool) -> String {
    if hour24 {
        chrono::Local::now().format("%H:%M").to_string()
    } else {
        chrono::Local::now().format("%I:%M %p").to_string()
    }
}

pub fn time_str(ts: i64) -> String {
    if ts <= 0 { return "—".to_string(); }
    chrono::DateTime::from_timestamp(ts, 0)
        .map(|d| d.with_timezone(&chrono::Local).format("%m-%d %H:%M").to_string())
        .unwrap_or_else(|| "—".to_string())
}
