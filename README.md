# ntrans

跨平台翻译助手（默认英文界面，可在设置中切换中文）。提供两种使用方式：

- `ntrans`：图形界面（Windows 上为 GUI 子系统，无控制台窗口）
- `ntrans-cli translate "文本" [--from] [--to] [--engine] [--lang en|zh]`：命令行翻译

功能特性：

- 翻译引擎：Google 免费接口（主）+ MyMemory（兜底），失败自动回退，可手动选择
- 源语言自动检测；中文/英文输入自动选择目标方向（可开关）
- 翻译历史记录（本地 JSON 持久化，可回填/复制/删除/清空）
- 系统托盘：显示/隐藏主窗口、设置、退出、关闭最小化到托盘（Windows/macOS）
- 多语言界面：英文（默认）/ 中文，中文字体运行时自动加载系统字体
- 跨平台：Windows / macOS / Linux

---

## 目录结构

```
src/
  main.rs           ntrans（GUI 入口，Windows 使用 GUI 子系统）
  lib.rs            核心库模块汇总
  bin/ntrans-cli.rs ntrans-cli（命令行入口）
  gui.rs            egui 界面（工具栏/控制条/编辑区/历史/设置）
  providers.rs      Google / MyMemory 翻译实现与容错
  model.rs          语言表、请求/结果模型、中英方向检测
  i18n.rs           界面文案表（英文/中文）与错误本地化
  history.rs        历史记录存取
  app_settings.rs   设置持久化（界面语言/方向/历史上限/托盘行为）
  fonts.rs          跨平台系统 CJK 字体加载
  icon.rs           纯代码生成的图标（RGBA）
  tray.rs           系统托盘（Windows/macOS；Linux 为空壳）
  paths.rs          跨平台数据目录定位
  ui_msg.rs         后台任务 → UI 消息
```

---

## 依赖的库（第三方 crates）

| Crate | 版本 | 用途 |
| --- | --- | --- |
| `eframe` / `egui` | 0.36 | GUI 框架与即时模式 UI |
| `reqwest` | 0.13 | HTTP 客户端（features: `json` `query` `native-tls` `system-proxy` `charset`） |
| `tokio` | 1 | 异步运行时（`rt-multi-thread` `macros` `time`） |
| `serde` / `serde_json` | 1 | 序列化（历史/设置 JSON） |
| `clap` | 4 | 命令行参数解析（derive） |
| `dirs` | 6 | 跨平台数据目录定位 |
| `thiserror` | 2 | 错误类型派生 |
| `chrono` | 0.4 | 历史记录本地时间戳 |
| `tray-icon` | 0.24 | 系统托盘（仅 Windows/macOS 目标，`default-features = false`） |

> 说明：`tray-icon` 为 Windows/macOS 目标专用依赖，Linux 上不编译。

---

## 环境要求

- Rust 工具链（stable，支持 edition 2024，建议 ≥ 1.85）
- Linux：编译原生 GUI 需要系统基础库（GTK3/wayland/x11 相关开发库随发行版自带即可，多数发行版可直接构建）；运行时中文显示需要系统已安装一种 CJK 字体（程序会自动探测常见路径）
- Windows：构建与运行无需额外依赖（程序静态链接 CRT）

---

## 编译命令

### 本机直接运行（调试）

```sh
cargo run            # 启动 GUI（ntrans）
cargo run --bin ntrans-cli -- translate "hello world" --to zh-CN
```

### 本机构建

```sh
cargo build                              # debug
cargo build --release                    # release（已开启 thin-LTO 与 strip）
cargo build --release --bin ntrans-cli   # 仅命令行程序
```

### 测试 / 静态检查

```sh
cargo test
cargo clippy --all-targets
cargo fmt
```

### 交叉编译 Windows 可执行文件

在 Linux 上使用 `cargo-xwin` + `lld-link` 交叉编译出 `ntrans.exe`（Windows GUI 子系统，静态链接 CRT，目标机无需安装 VC++ 运行库）。

前提：

```sh
# 1) 安装 Windows 目标
rustup target add x86_64-pc-windows-msvc

# 2) 安装 cargo-xwin（内含 xwin，自动下载 MSVC CRT / Windows SDK）
cargo install cargo-xwin

# 3) 提供 lld-link（随 LLVM 提供；Arch 等可用包管理器安装 lld）
export PATH="/path/to/llvm/bin:$PATH"     # 确保 lld-link 可执行
```

构建：

```sh
export PATH="$HOME/.cargo/bin:$PATH"
RUSTFLAGS='-C target-feature=+crt-static' \
  cargo xwin build --release --target x86_64-pc-windows-msvc
```

产物：

```
target/x86_64-pc-windows-msvc/release/ntrans.exe
target/x86_64-pc-windows-msvc/release/ntrans-cli.exe
```

- `ntrans.exe`：GUI 子系统，双击直接运行界面，无控制台弹窗
- `ntrans-cli.exe`：控制台程序，用于命令行翻译
  ```sh
  ntrans-cli translate "你好" --to en
  ntrans-cli translate "hello" --to zh-CN --lang zh
  ```

---

## 数据与配置位置

按平台存储于系统数据目录（`paths.rs` 中定位，目录名 `ntrans`）：

| 平台 | 路径（示例） |
| --- | --- |
| Windows | `%APPDATA%\ntrans\` |
| macOS | `~/Library/Application Support/ntrans/` |
| Linux | `$XDG_DATA_HOME/ntrans/`（默认 `~/.local/share/ntrans/`） |

- `settings.json`：界面语言、自动方向、历史上限、托盘行为
- `history.json`：翻译历史

---

## 翻译引擎说明

- 引擎「Auto」优先 Google 免费接口（`translate.googleapis.com`），失败自动回退 MyMemory
- 若网络无法访问 Google，请在界面手动切换引擎为 MyMemory
- 均为免费公开接口，无 API Key；可能受网络/限流影响，仅供学习与日常使用
