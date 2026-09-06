//! 语言与基础数据模型。

use crate::i18n::UiLang;

/// 一种受支持语言的代码（与 Google/MyMemory 上游兼容）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Lang {
    /// 上游语言代码，如 `auto`、`zh-CN`、`en`。
    pub code: &'static str,
    /// 界面展示名（英文）。
    pub label_en: &'static str,
    /// 界面展示名（中文）。
    pub label_zh: &'static str,
    /// 是否可作为目标语言。
    pub can_be_target: bool,
}

impl Lang {
    /// 按当前界面语言取展示名。
    pub fn name(self, lang: UiLang) -> &'static str {
        match lang {
            UiLang::En => self.label_en,
            UiLang::Zh => self.label_zh,
        }
    }
}

pub const AUTO: Lang = Lang {
    code: "auto",
    label_en: "Auto detect",
    label_zh: "自动检测",
    can_be_target: false,
};
pub const ZH: Lang = Lang {
    code: "zh-CN",
    label_en: "Chinese (Simplified)",
    label_zh: "简体中文",
    can_be_target: true,
};
pub const EN: Lang = Lang {
    code: "en",
    label_en: "English",
    label_zh: "英语",
    can_be_target: true,
};
pub const JA: Lang = Lang {
    code: "ja",
    label_en: "Japanese",
    label_zh: "日语",
    can_be_target: true,
};
pub const KO: Lang = Lang {
    code: "ko",
    label_en: "Korean",
    label_zh: "韩语",
    can_be_target: true,
};
pub const FR: Lang = Lang {
    code: "fr",
    label_en: "French",
    label_zh: "法语",
    can_be_target: true,
};
pub const DE: Lang = Lang {
    code: "de",
    label_en: "German",
    label_zh: "德语",
    can_be_target: true,
};
pub const ES: Lang = Lang {
    code: "es",
    label_en: "Spanish",
    label_zh: "西班牙语",
    can_be_target: true,
};
pub const RU: Lang = Lang {
    code: "ru",
    label_en: "Russian",
    label_zh: "俄语",
    can_be_target: true,
};
pub const IT: Lang = Lang {
    code: "it",
    label_en: "Italian",
    label_zh: "意大利语",
    can_be_target: true,
};
pub const PT: Lang = Lang {
    code: "pt",
    label_en: "Portuguese",
    label_zh: "葡萄牙语",
    can_be_target: true,
};
pub const AR: Lang = Lang {
    code: "ar",
    label_en: "Arabic",
    label_zh: "阿拉伯语",
    can_be_target: true,
};

pub const LANGS: &[Lang] = &[AUTO, ZH, EN, JA, KO, FR, DE, ES, RU, IT, PT, AR];

/// 由语言代码查语言，找不到时返回 `None`。
pub fn find(code: &str) -> Option<Lang> {
    LANGS.iter().copied().find(|l| l.code == code)
}

fn is_han(c: char) -> bool {
    matches!(c as u32, 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF | 0x20000..=0x2FA1F)
}

fn is_kana(c: char) -> bool {
    matches!(c as u32, 0x3040..=0x30FF)
}

/// 粗略判断文本是否为中文（含简体/繁体汉字）。
///
/// 规则：只要出现汉字且不含日文假名，即视为中文；否则视为非中文
/// （英文/日文/韩文等一律归为“其他语言”）。日文因大量使用汉字，
/// 以是否含假名作为区分，避免被误判成中文。
pub fn is_chinese(text: &str) -> bool {
    let mut han = false;
    let mut kana = false;
    for c in text.chars() {
        if is_han(c) {
            han = true;
        } else if is_kana(c) {
            kana = true;
        }
        if han && kana {
            break;
        }
    }
    han && !kana
}

/// 翻译请求。
#[derive(Clone, Debug)]
pub struct TranslateRequest {
    pub text: String,
    pub from: String,
    pub to: String,
}

/// 一次成功的翻译结果。
#[derive(Clone, Debug)]
pub struct TranslateOut {
    pub text: String,
    /// 实际检测/使用的源语言代码（`from == auto` 时由上游返回）。
    pub detected_from: String,
    pub engine: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_chinese() {
        assert!(is_chinese("你好，世界"));
        assert!(is_chinese("Hello 你好 world"));
        assert!(is_chinese("繁體中文測試"));
        assert!(!is_chinese("hello world"));
        assert!(!is_chinese("こんにちは"));
        assert!(!is_chinese("日本語のテスト"));
        assert!(!is_chinese("안녕하세요"));
        assert!(!is_chinese(""));
        assert!(!is_chinese("12345 !@#"));
    }
}
