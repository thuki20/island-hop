# IslandHop

屏幕顶部的一条「灵动岛」，把**局域网键鼠共享 + 剪贴板同步 + 桌面小工具**收进去。

- 形态学「岛仔」：收起是屏幕顶部一条小胶囊，鼠标移入展开、移出收起，置顶、无边框、透明。
- 能力来自 OpenHop：`crates/core` 是已验证的纯 Rust 引擎（键鼠捕获/注入、屏幕布局、设备发现、Noise 加密配对、剪贴板同步）。
- **没有 WebView2 / Electron / Tauri**：UI 用 egui 自绘，单个原生二进制。
- 双端：`macOS (arm64)` 与 `Windows (x64)`。

> **当前状态（2026-11-13 已出包）**：commit `de946c6` 已推送，Release `v0.1.0` 的两个资产都已替换成本轮新构建。
> macOS dmg 由本机 `packaging/macos-app.sh` 生成并上传；Windows zip 由 GitHub Actions 构建并上传（exe 已确认带 `.rsrc` 图标资源）。
> 旧包（无刘海适配、无图标、无功能页）已被覆盖，**需要重新下载安装才生效**。

---

## 1. 刘海适配（本轮修）

旧版只有 `with_always_on_top()`（macOS 上是 `NSFloatingWindowLevel=3`），**低于菜单栏 `NSMainMenuWindowLevel=24`**，所以岛被压在菜单栏下面，看起来「不在刘海上」。本轮：

- 移植旧 OpenHop 的做法：通过 `Frame` 的 raw window handle 拿 `NSView` → `[view window]` → `setLevel: 25`（`NSStatusWindowLevel`），并设 `collectionBehavior = 1|16|256`（所有 Space / stationary / 全屏辅助）。
- 用 `core::extras::system::notch()`（`safeAreaInsets` + 左右辅助区）算刘海几何，收起胶囊改成 `宽 = 刘海宽 + 18pt`、`高 = safeAreaInsets.top`、`y = 0`，正好把刘海包进去。
- AppKit 在启动早期可能查不到刘海，所以「没查到」不缓存、每 2 秒重试；查到后永久缓存。
- 无刘海机型 / Windows 自动回退成 236×40 胶囊，`y = 6`。

本机实测日志（`RUST_LOG=info` 跑 release 二进制 7 秒）：

```
notch detected: 185 x 32 pt
island window raised to NSStatusWindowLevel(25) with all-spaces collection behavior
island geometry: x=654 y=0 w=203 h=32 expanded=false
```

1512pt 宽屏幕中心 756 → 胶囊 `[654, 857]`，刘海 `[663, 848]`，已包住且贴顶。
**未做到的验证**：本机 `screencapture` 抓到的是全黑帧（显示器休眠/录屏权限），所以**没有目视截图确认**，上面是程序化证据。

## 2. 应用包与图标（本轮补）

| 平台 | 之前 | 现在 |
|---|---|---|
| macOS | `.app` 设了 `LSUIElement=true`（Dock 无图标）、`Info.plist` 没有 `CFBundleIconFile`，**看起来不像应用** | 去掉 `LSUIElement`、Dock 图标可见；`AppIcon.icns` 打进 `Contents/Resources`；`codesign` adhoc 签名 |
| Windows | 裸 `IslandHop.exe`，通用程序图标、无版本信息 | `app.rc` + `build.rs` 调 Windows SDK `rc.exe`，把 `AppIcon.ico` 和文件版本信息编进 exe（资源管理器/任务栏/Alt-Tab 有自定义图标） |

核实过岛仔：它确实有 exe 图标——仓库根目录 `app.rc` 引用 `assets/daozai.ico`，`build.rs` 调 `rc.exe` 编 `.res` 交给链接器，同时写 `FileDescription` / `ProductName` 等版本信息。本项目照这个思路自己实现，未复制其代码或图标。

图标由 `packaging/make-icon.py` 用标准库生成（深蓝圆角方 + 白色岛形胶囊 + 绿/蓝状态点），零第三方依赖：

```bash
python3 packaging/make-icon.py   # 生成 app/assets/AppIcon.{png,icns,ico}
bash packaging/macos-app.sh --app-only   # 只生成本地 .app，不产 dmg
bash packaging/macos-app.sh              # .app + dist/*.dmg（出包时用）
```

**本轮已执行打包**：`IslandHop.app`（带 `AppIcon.icns`、无 `LSUIElement`、Dock 图标可见）与 `dist/IslandHop-macOS-arm64.dmg` 已生成，`hdiutil verify` VALID；Windows zip 由 CI 产出，`unzip -t` 通过。

## 3. 功能页（按 C 方案：尽量复刻岛仔主力功能）

展开后顶部是页签，页面可关、可上下排序；设置页可换 5 套配色、刘海/胶囊、顶/左/右停靠、80–140% 缩放、12/24 小时、极简模式。

| 页面 | 本版真实能力 | 与岛仔的差距 |
|---|---|---|
| 跨屏 | 角色/配对码/共享口令/屏幕排列/剪贴板开关 + **连接诊断** | 无托盘、无自动更新 |
| 音乐 | 正在播放、播放/暂停/上一首/下一首（macOS: Spotify / 音乐；Windows: 系统媒体会话） | **无歌词**；Mac 上第三方播放器（网易云/B 站）读不到，是系统限制 |
| 天气 | Open-Meteo 当前实况 + 5 天预报，自动定位或填城市，15 分钟刷新、本机缓存 | 需本机 `curl` 可联网；无天气图标动画 |
| 待办 | 单次/每天/每周重复待办、番茄专注、饮水记录 | 提醒只在岛内显示，退出/睡眠期间不提醒（与岛仔相同）；无系统通知 |
| 置物 | 拖入文件临时收集、打开、定位、移除（不删原文件） | 无拖出、无跨机文件传输 UI |
| 剪贴板 | 最近 30 条文字历史、固定、点击写回、疑似密钥/口令不记录 | 不像岛仔能识别密码管理器来源；只是前缀规则 |
| 系统 | 本机 IP、端口、电量、磁盘、深色/勿扰、空闲 | 无音量 / CapsLock / 输入语言 / 蓝牙状态 |
| 额度 | **一页「未实现」诚实清单** | AI 额度、邮件验证码、微信/QQ 未读、远程日历全部**没有接入**，见下 |
| 设置 | 上述外观/页面管理 + 授权入口 + 重启/退出 | 皮肤是 egui 自绘近似配色，**没有原生毛玻璃** |

**明确未实现（不拿占位冒充）**：AI 额度（需各服务凭据/客户端本地缓存，且缺 Windows 凭据管理器/DPAPI 等价的安全存储，不引入明文存密钥）、邮件 IMAP 与验证码、微信/QQ 未读（岛仔靠 Windows 任务栏闪动，Mac 无对应接口）、远程日历（飞书/钉钉/CalDAV/EAS，连 `.ics` 导入也没做）、歌词、桌面伙伴动画、系统通知、原生毛玻璃、安装包/托盘/自动更新。岛仔本身也只出 Windows 包、明确不支持 macOS。

侧边停靠**只改变窗口位置**，收起胶囊仍是横向（未做竖排布局）；多屏仍只跟随第一块屏。

## 4. 「没受控」的诊断（Windows 还没测）

- 本机冒烟测试里引擎明确报了：`this computer's keyboard and mouse can't be shared: OpenHop needs Accessibility permission` / `other computers can't use this screen`。也就是说**即使 Windows 端连上，Mac 端没拿辅助功能权限就无法共享/被控**。该日志来自 `target/release/IslandHop` 这个裸二进制；你正在跑的 `.app` 是另一个路径，TCC 授权按 app 路径计，需要在系统设置里单独确认。
- 配置本身没问题：`role=server`、`enabled=true`、口令非空、已继承 `Windows` 配对、布局 Windows 在右、`last_ips=192.168.1.15`。`role=server` 表示 **Mac 是主控（把 Mac 键鼠共享出去）**，不是「Mac 被控」；想让 Mac 受控要把角色切成「受控」，Windows 端设为主控。
- 岛上「跨屏」页现在会直接显示：角色含义、已发现/已配对设备数、活动屏、端口、未配对原因、权限提示，以及「辅助功能授权 / 输入监控授权」一键跳转。**授权后必须彻底退出重开**。
- 双机端到端切换、剪贴板跨机同步**仍未实测**（Windows 端尚未安装/运行），这项不能报通过。

## 5. 数据位置与隐私

- 引擎配置：`~/Library/Application Support/island-hop/config.toml`（Windows `%APPDATA%\island-hop\config.toml`），首次启动自动继承旧 OpenHop。
- 界面数据：`ui.json`（皮肤、页面、剪贴板历史、待办、番茄、饮水、置物路径、天气缓存），**只在本机**。
- 天气会访问 `ipwho.is`（自动定位）与 `api.open-meteo.com` / `geocoding-api.open-meteo.com`；关掉天气页后不再自动请求。
- 邮箱/日历/AI 服务凭据：本版**不收集、不存储**（因为功能没做）。

## 6. 构建与发布

```bash
cargo build --release -p island-hop
```

- macOS：Xcode Command Line Tools；Windows：Rust stable + MSVC 生成工具（图标资源需要 Windows SDK 的 `rc.exe`，找不到只 warning 不中断）。
- 不需要 Node.js / WebView2 / NSIS / 任何前端工具链。
- CI（`.github/workflows/build.yml`）已改为**不再删除重建 Release**（旧版每次 push 都会清空 `v0.1.0` 资产）；只在 Release 不存在时创建。
- CI 的 `macos` job 在本账号下 push 时会被 GitHub 立刻取消（两次实测：0 step、1 秒、runner 为空），会把整次 run 标记成 `cancelled`。现已改为**仅 `workflow_dispatch` 手动触发**；push 只跑 `release` + `windows`，macOS 包固定由本机构建后上传。

本轮发布资产（Release `v0.1.0`，均为 2026-11-13 新构建）：

| 资产 | 大小 | 来源 | 校验 |
|---|---|---|---|
| `IslandHop-macOS-arm64.dmg` | 4,425,600 B | 本机 `macos-app.sh` | `hdiutil verify` VALID；内含 `AppIcon.icns`、arm64 Mach-O |
| `IslandHop-Windows-x64.zip` | 3,816,427 B | GitHub Actions `windows` job | `unzip -t` OK；`IslandHop.exe` 8,830,976 B，含 `.rsrc` 23,040 B（6 个尺寸图标 + 版本信息） |

## 7. 已知限制（如实说明）

- CI 只验证编译与打包；**双机端到端实测未完成**。
- 展开靠「鼠标进入窗口」触发，不是全局监听屏幕顶部边缘。
- 中文 UI 依赖运行时加载系统字体（PingFang / 微软雅黑），找不到时**主框架文案**降级英文，功能页内的说明文案仍是中文。
- adhoc 签名，TCC 授权可能反复弹窗；Windows SmartScreen 可能提示「未知发布者」。
- 未做商业代码签名；`core` 里的日志文案仍写「OpenHop」（引擎原始文案，未改）。

## 许可

MIT。第三方声明见 `THIRD_PARTY_NOTICES.md`。
