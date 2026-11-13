//! 页签、设置页、「未实现功能」诚实清单。

use crate::state::*;
use crate::windowing;
use eframe::egui::{self, Color32, RichText};
use openhop_core::Role;

pub enum Action {
    Role(Role),
    SaveName,
    SavePass,
    Pair(String, String),
    Place(String, i32),
    Clipboard(bool),
    Restart,
    Quit,
    OpenConfig,
    OpenPrivacy(String),
}

pub fn tab_bar(ui: &mut egui::Ui, data: &mut UiData, pal: Palette, cjk: bool) -> PageId {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for p in data.settings.pages.clone() {
            let on = data.settings.selected == p;
            let txt = RichText::new(p.label(cjk)).size(11.5)
                .color(if on { pal.accent } else { pal.muted }).strong();
            if ui.selectable_label(on, txt).clicked() {
                data.settings.selected = p;
            }
        }
    });
    ui.add_space(4.0);
    ui.separator();
    ui.add_space(4.0);
    data.settings.selected
}

pub fn quota_ui(ui: &mut egui::Ui, pal: Palette, cjk: bool) {
    ui.label(RichText::new(if cjk { "额度与消息 · 本版状态" } else { "Quota & messages · status" })
        .size(12.5).strong().color(pal.fg));
    ui.add_space(4.0);
    ui.label(RichText::new(if cjk {
        "这一页不显示任何数字，因为本版确实没有接入下列服务。岛仔的这些能力依赖 Windows 专用接口或各服务凭据；本版双端一致，不做假数据。"
    } else {
        "No numbers here: none of these services are wired up in this build."
    }).size(11.0).color(Color32::from_rgb(240, 190, 110)));
    ui.add_space(6.0);

    let rows: &[(&str, &str)] = &[
        ("AI 额度（Codex / Claude / WorkBuddy / DeepSeek / Gemini / Kimi…）",
         "未实现：需各服务 API Key / 客户端本地缓存；岛仔用 Windows 凭据管理器与 DPAPI 保管，本版没有等价的安全存储，不引入明文存密钥的口子。"),
        ("邮件与验证码（IMAP）", "未实现：需邮箱授权码与邮件正文处理，涉及隐私与凭据存储。"),
        ("微信 / QQ 未读", "未实现：岛仔靠 Windows 任务栏闪动识别，macOS 无对应接口；双端一致的方案需另做。"),
        ("远程日历（飞书 / 钉钉 / CalDAV / EAS）", "未实现：需服务端凭据与同步；本版也没有 .ics 导入。"),
        ("歌词", "未实现：只做了「正在播放」信息与控制，没有歌词源匹配。"),
        ("桌面伙伴 / 原生毛玻璃 / 系统通知", "未实现：皮肤是 egui 自绘的近似配色，没有原生毛玻璃、伙伴动画和系统通知。"),
        ("安装包 / 托盘 / 自动更新", "未实现：本轮按你的要求「现在不出包」，只把应用包与图标配置补进工程。"),
    ];
    egui::ScrollArea::vertical().max_height(290.0).show(ui, |ui| {
        for (k, v) in rows {
            ui.label(RichText::new(*k).size(11.5).strong().color(pal.fg));
            ui.label(RichText::new(*v).size(10.5).color(pal.muted));
            ui.add_space(5.0);
        }
        if !cjk {
            ui.label(RichText::new("(Details are written in Chinese in this build.)").size(10.0).color(pal.muted));
        }
    });
}

pub fn settings_ui(ui: &mut egui::Ui, data: &mut UiData, pal: Palette, cjk: bool) -> Vec<Action> {
    let mut acts = Vec::new();
    let s = &mut data.settings;

    ui.label(RichText::new(if cjk { "岛体外观" } else { "Appearance" }).size(12.5).strong().color(pal.fg));
    ui.add_space(3.0);
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(if cjk { "皮肤" } else { "Skin" }).size(11.0).color(pal.muted));
        for sk in Skin::all() {
            if ui.selectable_label(s.skin == sk, RichText::new(sk.label()).size(11.0)).clicked() {
                s.skin = sk;
            }
        }
    });
    ui.horizontal(|ui| {
        ui.label(RichText::new(if cjk { "形态" } else { "Shape" }).size(11.0).color(pal.muted));
        for sh in [Shape::Notch, Shape::Capsule] {
            if ui.selectable_label(s.shape == sh, RichText::new(sh.label()).size(11.0)).clicked() { s.shape = sh; }
        }
        ui.add_space(8.0);
        ui.label(RichText::new(if cjk { "停靠" } else { "Dock" }).size(11.0).color(pal.muted));
        for d in [Dock::Top, Dock::Left, Dock::Right] {
            if ui.selectable_label(s.dock == d, RichText::new(d.label()).size(11.0)).clicked() { s.dock = d; }
        }
    });
    ui.horizontal(|ui| {
        ui.add(egui::Slider::new(&mut s.scale, 0.8..=1.4).step_by(0.05).text(if cjk { "大小" } else { "Scale" }));
        ui.checkbox(&mut s.hour24, if cjk { "24 小时制" } else { "24h" });
        ui.checkbox(&mut s.minimal, if cjk { "极简模式" } else { "Minimal" });
    });
    let notch = windowing::notch_size();
    ui.label(RichText::new(match notch {
        Some((w, h)) => format!("检测到刘海：{w:.0} × {h:.0} pt（刘海形态会贴到 y=0，并抬到菜单栏之上）"),
        None => "未检测到刘海：岛上移不了，只能按胶囊形态贴在顶部；Windows 无刘海属正常。".into(),
    }).size(10.5).color(pal.muted));
    ui.label(RichText::new(if cjk {
        "侧边停靠只改变停靠位置，收起胶囊仍是横向（未做竖排布局）。"
    } else { "Side docks only move the window; the pill stays horizontal." })
        .size(10.5).color(pal.muted));
    ui.add_space(6.0);
    ui.separator();

    ui.label(RichText::new(if cjk { "功能页（可关、可排序）" } else { "Pages" }).size(12.5).strong().color(pal.fg));
    ui.add_space(3.0);
    let mut move_up: Option<usize> = None;
    let mut move_down: Option<usize> = None;
    let mut toggle: Option<PageId> = None;
    let snapshot = s.pages.clone();
    for (i, p) in snapshot.iter().enumerate() {
        ui.horizontal(|ui| {
            let mut on = true;
            if ui.checkbox(&mut on, "").clicked() { toggle = Some(*p); }
            ui.label(RichText::new(p.label(cjk)).size(11.0).color(pal.fg));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("↓").clicked() { move_down = Some(i); }
                if ui.small_button("↑").clicked() { move_up = Some(i); }
            });
        });
    }
    {
        let pages = &mut s.pages;
        if let Some(p) = toggle {
            // 设置页不允许隐藏，否则没有入口把其它页加回来。
            if p != PageId::Settings {
                if pages.contains(&p) {
                    pages.retain(|x| *x != p);
                } else {
                    pages.push(p);
                }
            }
        }
        if !pages.contains(&PageId::Settings) {
            pages.push(PageId::Settings);
        }
        if let Some(i) = move_up { if i > 0 { pages.swap(i - 1, i); } }
        if let Some(i) = move_down { if i + 1 < pages.len() { pages.swap(i, i + 1); } }
        if !pages.contains(&s.selected) {
            s.selected = pages.first().copied().unwrap_or(PageId::Settings);
        }
    }
    ui.horizontal(|ui| {
        ui.label(RichText::new(if cjk { "未显示的页" } else { "Hidden" }).size(10.5).color(pal.muted));
        for p in PageId::all() {
            if !s.pages.contains(&p) && ui.small_button(format!("+{}", p.label(cjk))).clicked() {
                s.pages.push(p);
            }
        }
    });
    ui.add_space(6.0);
    ui.separator();

    ui.label(RichText::new(if cjk { "计时与饮水" } else { "Timers" }).size(12.5).strong().color(pal.fg));
    ui.horizontal(|ui| {
        ui.add(egui::Slider::new(&mut s.focus_min, 1..=90).text(if cjk { "专注 min" } else { "Focus" }));
        ui.add(egui::Slider::new(&mut s.break_min, 1..=30).text(if cjk { "休息 min" } else { "Break" }));
    });
    ui.horizontal(|ui| {
        ui.add(egui::Slider::new(&mut s.water_goal, 250..=4000).step_by(250.0).text(if cjk { "目标 ml" } else { "Goal" }));
        ui.add(egui::Slider::new(&mut s.water_cup, 50..=1000).step_by(50.0).text(if cjk { "每杯 ml" } else { "Cup" }));
    });
    ui.add_space(6.0);
    ui.separator();

    ui.horizontal_wrapped(|ui| {
        if ui.small_button(if cjk { "打开配置目录" } else { "Open config folder" }).clicked() {
            acts.push(Action::OpenConfig);
        }
        if ui.small_button(if cjk { "辅助功能授权" } else { "Accessibility" }).clicked() {
            acts.push(Action::OpenPrivacy("accessibility".into()));
        }
        if ui.small_button(if cjk { "输入监控授权" } else { "Input monitoring" }).clicked() {
            acts.push(Action::OpenPrivacy("input".into()));
        }
        if ui.small_button(if cjk { "重启引擎" } else { "Restart engine" }).clicked() {
            acts.push(Action::Restart);
        }
        if ui.small_button(if cjk { "退出" } else { "Quit" }).clicked() {
            acts.push(Action::Quit);
        }
    });
    ui.add_space(4.0);
    ui.label(RichText::new("IslandHop v0.1.0+ · MIT · 引擎来自 OpenHop (MIT)").size(10.0).color(pal.muted));
    acts
}
