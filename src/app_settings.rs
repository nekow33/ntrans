//! 应用设置持久化。

use serde::{Deserialize, Serialize};

use crate::{i18n::UiLang, paths};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// 关闭主窗口时隐藏到托盘（仅支持托盘的系统生效）。
    pub hide_on_close: bool,
    /// 历史记录保留条数。
    pub history_capacity: usize,
    /// 自动方向：输入中文→译英，其他→译中。
    pub auto_direction: bool,
    /// 界面语言（默认英文）。
    pub ui_lang: UiLang,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hide_on_close: true,
            history_capacity: 200,
            auto_direction: true,
            ui_lang: UiLang::En,
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        let mut s: Settings = paths::load_json("settings.json", Settings::default());
        s.history_capacity = s.history_capacity.clamp(1, 10_000);
        s
    }

    pub fn save(&self) {
        if let Err(e) = paths::save_json("settings.json", self) {
            eprintln!("保存设置失败: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_sane() {
        let s = Settings::default();
        assert!(s.hide_on_close);
        assert_eq!(s.history_capacity, 200);
        assert!(s.auto_direction);
        assert_eq!(s.ui_lang, UiLang::En);
    }
}
