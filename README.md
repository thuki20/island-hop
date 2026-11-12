# IslandHop

屏幕顶部的一条「灵动岛」，把**局域网键鼠共享 + 剪贴板同步**收进去。

- 形态学「岛仔」：收起是屏幕顶部一条小胶囊，鼠标移入展开、移出收起，置顶、无边框、透明。
- 能力来自 OpenHop：`crates/core` 是已验证的纯 Rust 引擎（键鼠捕获/注入、屏幕布局、设备发现、Noise 加密配对、剪贴板同步）。
- **没有 WebView2 / Electron / Tauri**：UI 用 egui 自绘，单个原生二进制。
- 双端：`macOS (arm64)` 与 `Windows (x64)`，两端都装这个软件即可互通。

## 下载与安装

Release `v0.1.0` 提供两个资产：

| 平台 | 文件 | 安装 |
|---|---|---|
| Windows x64 | `IslandHop-Windows-x64.zip` | 解压出 `IslandHop.exe`，放到固定目录，双击运行（可建快捷方式开机自启） |
| macOS arm64 | `IslandHop-macOS-arm64.dmg` | 打开 dmg，把 `IslandHop.app` 拖进「应用程序」 |

> 两端都装、都运行，才算「结合重装」。旧 OpenHop 可以先留着，但**不要和 IslandHop 同时运行**（会抢 24850/24851 端口与输入钩子）。

### macOS 首次运行（必看）

1. 岛是 `LSUIElement`，**没有 Dock 图标**，退出请用岛上的「退出」按钮。
2. 首次启动会请求**辅助功能 / 输入监控**授权：系统设置 → 隐私与安全性 → 辅助功能、输入监控，勾选 IslandHop 后**彻底退出并重新打开**才生效。
3. CI 产物是 **adhoc 签名**（无开发者证书）。按以往经验，adhoc 签名的 TCC 授权可能反复弹窗。若要稳定，请在本机用自签代码签名证书重签：
   ```bash
   codesign --force --deep --sign "OpenHopSign" /Applications/IslandHop.app
   ```
   重签后重新授权一次即可。

## 使用

1. 两端都启动。想让哪台机器**提供物理键鼠**，就在那台的岛上选「主控（共享键鼠）」，另一台选「受控」。
2. 主控端的岛展开后显示 **6 位配对码**；受控端展开后输入该码，点对应设备的「配对」。
   - 也可以在两端填同一个**共享口令**（≥4 字符）免配对。
3. 在「屏幕排列」里给对端设「设为左 / 设为右」，与实际物理位置一致，鼠标移到边缘才能正确切换。
4. 剪贴板跨机同步默认开启，可在岛上关掉。

配置写在 `%APPDATA%\island-hop\config.toml`（Windows）/ `~/Library/Application Support/island-hop/config.toml`（macOS）。**首次启动会自动继承旧 OpenHop 的配置**（设备名、口令、配对关系、屏幕布局），所以重装通常不用重新配对。

## 与旧 OpenHop 的关系

- 协议完全兼容（v15，UDP 24851 发现 / TCP 24850 Noise 加密），所以也可以「一端 IslandHop、一端旧 OpenHop」混用。
- 但本项目的定位是**两端都换成 IslandHop**：UI 更轻、双端一致。
- `crates/core` 复制自 OpenHop（MIT），保留了原有版权声明；详见 `THIRD_PARTY_NOTICES.md`。

## 已知限制（如实说明）

- CI 只验证**编译与打包**；跨屏切换、剪贴板同步的**双机端到端实测尚未在本仓库完成**，需要你在 Mac + Windows 上实机验收。
- 岛的展开目前靠「鼠标进入窗口」触发，不是全局监听屏幕顶部边缘（比岛仔略简化）。
- 收起/展开时窗口尺寸变化，多显示器下只跟随第一块屏。
- 没有做自动更新、托盘菜单、文件传输 UI（core 支持，UI 未暴露）。
- 未做商业代码签名，Windows SmartScreen 可能提示「未知发布者」。
- 中文 UI 依赖运行时加载系统字体（macOS PingFang / Windows 微软雅黑）；找不到时文案自动降级为英文，不会乱码。

## 从源码构建

```bash
cargo build --release -p island-hop
```

- macOS：需要 Xcode Command Line Tools。
- Windows：需要 Rust stable + MSVC 生成工具。
- 两端都不需要 Node.js、WebView2 或 NSIS。

## 许可

MIT。第三方声明见 `THIRD_PARTY_NOTICES.md`。
