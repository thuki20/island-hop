# 第三方声明

## OpenHop（代码来源）

`crates/core/` 复制自 OpenHop（`https://github.com/maykano-dev/open-hop`，用户 fork `thuki20/openhop-zh`），
按 MIT 许可使用，保留原版权与许可声明。包含：键鼠捕获/注入平台层、屏幕布局、
局域网设备发现、Noise 加密传输、配对、剪贴板与文件传输协议。

本仓库对 `crates/core` 的改动：仅作为 workspace 成员被 `app` 依赖，未修改其源码逻辑。

## 岛仔 Daozai（设计参考，未使用代码）

`https://github.com/Renly1994/xiaotang-daozai` — MIT。
本项目**只借鉴交互形态与信息密度思路**（屏幕顶部胶囊、悬停展开/离开收起、
功能页组织方式，以及「用 app.rc + rc.exe 给 exe 嵌图标」这一工程做法），
**未复制其任何源码、图标、素材、字体或品牌标识**。
本项目的 `app/app.rc` 与 `app/build.rs` 是自行编写的等价实现；
`app/assets/AppIcon.*` 由 `packaging/make-icon.py` 用 Python 标准库生成。
其名称、Logo、桌面伙伴形象均不随 MIT 许可授予，本项目不使用。
岛仔为 Windows-only，本项目的 macOS 端功能与它没有代码关系。

## 在线数据服务（天气页）

- Open-Meteo（`api.open-meteo.com` / `geocoding-api.open-meteo.com`）：天气与城市地理编码，免费、无需密钥。
- ipwho.is：仅用于「留空城市时按网络出口自动定位」，只取经纬度与城市名。

天气页通过本机 `curl` 命令行访问上述服务；关闭天气页后不再自动请求。
不使用任何需要用户登录或凭据的服务。

## 依赖

所有 Rust 依赖来自 crates.io，沿用各自许可（MIT / Apache-2.0 为主）。
主要直接依赖：`eframe`/`egui`（MIT/Apache-2.0）、`openhop-core`（本仓库）、
`dirs`、`chrono`、`log`、`env_logger`、`anyhow`。
完整依赖树见 `Cargo.lock`。
