//! 翻译助手核心库：GUI 与 CLI 共享的业务代码。
//!
//! 二进制入口：
//! - `src/main.rs`     → `my`（图形界面）
//! - `src/bin/ntrans-cli.rs` → `ntrans-cli`（命令行翻译）

pub mod app_settings;
pub mod cli;
pub mod fonts;
pub mod gui;
pub mod history;
pub mod i18n;
pub mod icon;
pub mod model;
pub mod paths;
pub mod providers;
pub mod tray;
pub mod ui_msg;
