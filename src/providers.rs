//! 翻译上游实现：Google 网页接口（主）+ MyMemory（兜底）。

use std::time::Duration;

use serde_json::Value;

use crate::i18n::{self, UiLang};
use crate::model::{TranslateOut, TranslateRequest};

/// 统一错误类型。
#[derive(Debug, thiserror::Error)]
pub enum TrError {
    #[error("网络请求失败: {0}")]
    Http(#[from] reqwest::Error),
    #[error("解析上游响应失败: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Msg(String),
    #[error("upstream returned empty content")]
    Empty(UiLang),
}

impl TrError {
    /// 转成本界面语言的字符串。
    pub fn to_localized(&self, lang: UiLang) -> String {
        match self {
            TrError::Empty(_) => i18n::err_empty(lang).to_owned(),
            _ => self.to_string(),
        }
    }
}

type Result<T> = std::result::Result<T, TrError>;

/// 可选翻译引擎。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Engine {
    /// 自动：优先 Google，失败时回退 MyMemory。
    #[default]
    Auto,
    Google,
    MyMemory,
}

impl Engine {
    pub const ALL: [Engine; 3] = [Engine::Auto, Engine::Google, Engine::MyMemory];

    pub fn label(self) -> &'static str {
        match self {
            Engine::Auto => "自动（Google→MyMemory）",
            Engine::Google => "Google",
            Engine::MyMemory => "MyMemory",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Engine::Auto => "auto",
            Engine::Google => "google",
            Engine::MyMemory => "mymemory",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        Engine::ALL.iter().copied().find(|e| e.code() == code)
    }
}

const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36";

/// 执行一次翻译。
pub async fn translate(
    req: &TranslateRequest,
    engine: Engine,
    lang: UiLang,
) -> Result<TranslateOut> {
    if req.text.trim().is_empty() {
        return Err(TrError::Msg(i18n::err_input_empty(lang).into()));
    }
    let (out, engine_used) = match engine {
        Engine::Google => (google_translate(req, lang).await, "Google"),
        Engine::MyMemory => (mymemory_translate(req, lang).await, "MyMemory"),
        Engine::Auto => match google_translate(req, lang).await {
            Ok(o) => (Ok(o), "Google"),
            Err(_) => (mymemory_translate(req, lang).await, "MyMemory"),
        },
    };
    let out = out?;
    Ok(TranslateOut {
        detected_from: if req.from == "auto" {
            out.1.unwrap_or_else(|| "auto".into())
        } else {
            req.from.clone()
        },
        text: out.0,
        engine: engine_used,
    })
}

fn new_client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(20))
        .build()?)
}

/// Google `translate_a/single`（client=gtx）接口。
async fn google_translate(
    req: &TranslateRequest,
    lang: UiLang,
) -> Result<(String, Option<String>)> {
    let client = new_client()?;
    let url = "https://translate.googleapis.com/translate_a/single";
    let params = [
        ("client", "gtx"),
        ("sl", req.from.as_str()),
        ("tl", req.to.as_str()),
        ("dt", "t"),
        ("q", req.text.as_str()),
    ];
    let resp = client.get(url).query(&params).send().await?;
    let status = resp.status();
    let body = resp.text().await?;
    if !status.is_success() {
        return Err(TrError::Msg(i18n::err_google_status(lang, status)));
    }
    let trimmed = body.trim_start();
    if !trimmed.starts_with('[') {
        return Err(TrError::Msg(i18n::err_google_not_json(lang).into()));
    }
    parse_google(&body, lang)
}

fn parse_google(body: &str, lang: UiLang) -> Result<(String, Option<String>)> {
    let v: Value = serde_json::from_str(body)?;
    let segs = v
        .get(0)
        .and_then(Value::as_array)
        .ok_or_else(|| TrError::Msg(i18n::err_google_bad_response(lang).into()))?;
    let mut text = String::new();
    for seg in segs {
        if let Some(t) = seg.get(0).and_then(Value::as_str) {
            text.push_str(t);
        }
    }
    if text.is_empty() {
        return Err(TrError::Empty(lang));
    }
    let detected = v.get(2).and_then(Value::as_str).map(|s| s.to_owned());
    Ok((text, detected))
}

/// MyMemory 接口。
async fn mymemory_translate(
    req: &TranslateRequest,
    lang: UiLang,
) -> Result<(String, Option<String>)> {
    let client = new_client()?;
    let url = "https://api.mymemory.translated.net/get";
    let from = if req.from == "auto" {
        "Autodetect"
    } else {
        req.from.as_str()
    };
    let params = [
        ("q", req.text.as_str()),
        ("langpair", &format!("{from}|{}", req.to)),
    ];
    let resp = client.get(url).query(&params).send().await?;
    let status = resp.status();
    let body = resp.text().await?;
    if !status.is_success() {
        return Err(TrError::Msg(i18n::err_mymemory_status(lang, status)));
    }
    parse_mymemory(&body, lang)
}

fn parse_mymemory(body: &str, lang: UiLang) -> Result<(String, Option<String>)> {
    let v: Value = serde_json::from_str(body)?;
    let data = &v["responseData"];
    let translated = data["translatedText"].as_str().unwrap_or("").to_owned();
    let detected = data["detectedLanguage"].as_str().map(|s| s.to_owned());
    if translated.trim().is_empty() {
        let status = v["responseStatus"].as_i64().unwrap_or(0);
        let detail = v["responseDetails"].as_str().unwrap_or("").to_owned();
        return Err(TrError::Msg(i18n::err_mymemory_fail(lang, status, &detail)));
    }
    Ok((translated, detected))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_google_ok() {
        // 真实 Google 响应结构的一个片段（fixture）。
        let body =
            r#"[[["你好，世界","hello, world",null,null,10]],null,"en",null,null,null,null,[]]"#;
        let (text, detected) = parse_google(body, UiLang::En).unwrap();
        assert_eq!(text, "你好，世界");
        assert_eq!(detected.as_deref(), Some("en"));
    }

    #[test]
    fn parse_google_multiseg() {
        let body =
            r#"[[["你好","hello",null,null,1],["，世界","，world",null,null,1]],null,"en",null]"#;
        let (text, detected) = parse_google(body, UiLang::En).unwrap();
        assert_eq!(text, "你好，世界");
        assert_eq!(detected.as_deref(), Some("en"));
    }

    #[test]
    fn parse_google_rejects_html() {
        let body = "<html><body>Sorry</body></html>";
        assert!(parse_google(body, UiLang::En).is_err());
    }

    #[test]
    fn parse_mymemory_ok() {
        let body = r#"{"responseData":{"translatedText":"你好世界","match":1,"detectedLanguage":"en"},"responseStatus":200,"responseDetails":""}"#;
        let (text, detected) = parse_mymemory(body, UiLang::En).unwrap();
        assert_eq!(text, "你好世界");
        assert_eq!(detected.as_deref(), Some("en"));
    }

    #[test]
    fn parse_mymemory_empty() {
        let body = r#"{"responseData":{"translatedText":""},"responseStatus":500,"responseDetails":"no match"}"#;
        assert!(parse_mymemory(body, UiLang::En).is_err());
    }

    #[test]
    fn engine_code_roundtrip() {
        assert_eq!(Engine::from_code("mymemory"), Some(Engine::MyMemory));
        assert_eq!(Engine::from_code("auto"), Some(Engine::Auto));
        assert_eq!(Engine::from_code("nope"), None);
    }
}
