//! 界面多语言（UI texts + 本地化错误消息）。
//!
//! 默认英文界面；可在设置中选择中文。文案均为 `&'static str`，
//! 通过 [`Texts`] 按语言取用。

/// 界面语言。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UiLang {
    #[default]
    En,
    Zh,
}

impl UiLang {
    pub fn code(self) -> &'static str {
        match self {
            UiLang::En => "en",
            UiLang::Zh => "zh",
        }
    }

    pub fn from_code(code: &str) -> UiLang {
        match code {
            "zh" => UiLang::Zh,
            _ => UiLang::En,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            UiLang::En => "English",
            UiLang::Zh => "中文",
        }
    }

    pub fn texts(self) -> &'static Texts {
        match self {
            UiLang::En => &EN,
            UiLang::Zh => &ZH,
        }
    }
}

/// 界面上用到的所有文案。
pub struct Texts {
    pub title: &'static str,
    // 顶栏
    pub engine: &'static str,
    pub auto_engine: &'static str,
    pub from: &'static str,
    pub to: &'static str,
    pub swap: &'static str,
    pub translate: &'static str,
    pub translating: &'static str,
    pub translate_shortcut: &'static str,
    pub history: &'static str,
    pub settings: &'static str,
    pub auto_direction: &'static str,
    pub auto_direction_hint: &'static str,
    // 编辑区
    pub source: &'static str,
    pub target: &'static str,
    pub clear: &'static str,
    pub copy: &'static str,
    pub use_as_source: &'static str,
    pub input_hint: &'static str,
    pub output_hint: &'static str,
    pub no_translation_yet: &'static str,
    // 状态栏
    pub ready: &'static str,
    // 历史
    pub history_title: &'static str, // 含占位符 {n}
    pub clear_all: &'static str,
    pub history_empty: &'static str,
    pub fill_back: &'static str,
    pub copy_translation: &'static str,
    pub delete: &'static str,
    // 设置
    pub section_language: &'static str,
    pub ui_language: &'static str,
    pub section_general: &'static str,
    pub section_tray: &'static str,
    pub hide_on_close: &'static str,
    pub no_tray_note: &'static str,
    pub section_history: &'static str,
    pub keep_max: &'static str,
    pub entries_suffix: &'static str,
    pub engine_note: &'static str,
    pub language_note: &'static str,
}

use UiLang::{En, Zh};

pub const EN: Texts = Texts {
    title: "ntrans",
    engine: "Engine:",
    auto_engine: "Auto (Google → MyMemory)",
    from: "From:",
    to: "To:",
    swap: "Swap",
    translate: "Translate",
    translating: "Translating…",
    translate_shortcut: "Ctrl/Cmd+Enter to translate",
    history: "History",
    settings: "Settings",
    auto_direction: "Auto direction",
    auto_direction_hint: "Translate Chinese input to English, other input to Chinese (source must be Auto)",
    source: "Text",
    target: "Translation",
    clear: "Clear",
    copy: "Copy",
    use_as_source: "Use as input",
    input_hint: "Type text to translate… (Ctrl/Cmd + Enter)",
    output_hint: "Translation appears here",
    no_translation_yet: "No translation yet",
    ready: "Ready",
    history_title: "History ({n})",
    clear_all: "Clear all",
    history_empty: "(empty)",
    fill_back: "Refill",
    copy_translation: "Copy",
    delete: "Delete",
    section_language: "Interface language",
    ui_language: "Language",
    section_general: "General",
    section_tray: "System tray",
    hide_on_close: "Hide to tray when closing the window",
    no_tray_note: "No system tray on this platform; closing the window exits the app.",
    section_history: "History",
    keep_max: "Keep at most",
    entries_suffix: "entries",
    engine_note: "Engine “Auto” prefers the free Google endpoint and falls back to MyMemory.\n\
                  If Google is unreachable on your network, pick MyMemory manually.",
    language_note: "Restart the app for full language switch of remaining system messages.",
};

pub const ZH: Texts = Texts {
    title: "ntrans",
    engine: "引擎:",
    auto_engine: "自动（Google → MyMemory）",
    from: "从:",
    to: "到:",
    swap: "交换语言",
    translate: "翻译",
    translating: "翻译中…",
    translate_shortcut: "Ctrl/Cmd+Enter 翻译",
    history: "历史",
    settings: "设置",
    auto_direction: "自动方向",
    auto_direction_hint: "输入中文自动译成英文，其他语言自动译成中文（源语言需为“自动”）",
    source: "原文",
    target: "译文",
    clear: "清空",
    copy: "复制",
    use_as_source: "设为原文",
    input_hint: "输入要翻译的文本…（Ctrl/Cmd + Enter 翻译）",
    output_hint: "译文将显示在这里",
    no_translation_yet: "暂无译文",
    ready: "就绪",
    history_title: "历史记录 ({n})",
    clear_all: "清空",
    history_empty: "（暂无历史）",
    fill_back: "回填",
    copy_translation: "复制译文",
    delete: "删除",
    section_language: "界面语言",
    ui_language: "语言",
    section_general: "常规",
    section_tray: "托盘",
    hide_on_close: "关闭主窗口时最小化到托盘",
    no_tray_note: "当前平台无系统托盘，关闭窗口将直接退出应用。",
    section_history: "历史记录",
    keep_max: "最多保留",
    entries_suffix: "条",
    engine_note: "引擎「自动」优先使用 Google 免费接口，失败自动回退 MyMemory；\n\
                  若你的网络无法访问 Google，可手动切换为 MyMemory。",
    language_note: "部分系统消息在重启应用后完全切换语言。",
};

/// 引擎下拉/记录的显示名。
pub fn engine_display(engine: crate::providers::Engine, lang: UiLang) -> &'static str {
    use crate::providers::Engine as E;
    match engine {
        E::Auto => lang.texts().auto_engine,
        E::Google => "Google",
        E::MyMemory => "MyMemory",
    }
}

// ---------- 本地化的错误/提示消息 ----------

pub fn err_input_empty(lang: UiLang) -> &'static str {
    match lang {
        En => "Please enter some text to translate.",
        Zh => "请输入需要翻译的内容",
    }
}

pub fn err_google_status(lang: UiLang, status: reqwest::StatusCode) -> String {
    match lang {
        En => format!("Google returned status {status}"),
        Zh => format!("Google 返回状态 {status}"),
    }
}

pub fn err_google_not_json(lang: UiLang) -> &'static str {
    match lang {
        En => "Google returned unexpected content (possibly blocked or rate-limited).",
        Zh => "Google 返回了非 JSON 内容（可能被网络屏蔽或限流）",
    }
}

pub fn err_google_bad_response(lang: UiLang) -> &'static str {
    match lang {
        En => "Unexpected Google response structure.",
        Zh => "Google 响应结构异常",
    }
}

pub fn err_mymemory_status(lang: UiLang, status: reqwest::StatusCode) -> String {
    match lang {
        En => format!("MyMemory returned status {status}"),
        Zh => format!("MyMemory 返回状态 {status}"),
    }
}

pub fn err_mymemory_fail(lang: UiLang, status: i64, detail: &str) -> String {
    match lang {
        En => format!("MyMemory translation failed (status={status}, details={detail})"),
        Zh => format!("MyMemory 翻译失败 (status={status}, details={detail})"),
    }
}

pub fn err_empty(lang: UiLang) -> &'static str {
    match lang {
        En => "Upstream returned empty content.",
        Zh => "上游返回内容为空",
    }
}

pub fn err_unknown_engine(name: String, lang: UiLang) -> String {
    match lang {
        En => format!("Unknown engine: {name} (options: auto/google/mymemory)"),
        Zh => format!("未知引擎：{name}（可选 auto/google/mymemory）"),
    }
}

pub fn err_runtime(msg: String, lang: UiLang) -> String {
    match lang {
        En => format!("Failed to create runtime: {msg}"),
        Zh => format!("运行时创建失败: {msg}"),
    }
}
