//! 命令行翻译子命令：`ntrans-cli translate "文本"`。

use clap::Args;

use crate::i18n::{self, UiLang};
use crate::model::TranslateRequest;
use crate::providers::{Engine, translate};

#[derive(Debug, Args)]
pub struct TranslateArgs {
    /// 要翻译的文本。
    #[arg(required = true)]
    pub text: String,

    /// 源语言代码（默认 auto 自动检测）。
    #[arg(long, default_value = "auto")]
    pub from: String,

    /// 目标语言代码（默认 zh-CN）。
    #[arg(long, default_value = "zh-CN")]
    pub to: String,

    /// 翻译引擎：auto | google | mymemory（默认 auto）。
    #[arg(long, default_value = "auto")]
    pub engine: String,

    /// 仅输出译文（便于脚本使用）。
    #[arg(long)]
    pub quiet: bool,

    /// 输出语言：en | zh（默认 en）。
    #[arg(long, default_value = "en")]
    pub lang: String,
}

pub fn run(args: TranslateArgs) -> i32 {
    let Some(engine) = Engine::from_code(&args.engine.to_ascii_lowercase()) else {
        eprintln!(
            "{}",
            i18n::err_unknown_engine(args.engine.clone(), UiLang::from_code(&args.lang))
        );
        return 2;
    };
    let lang = UiLang::from_code(&args.lang);
    let req = TranslateRequest {
        text: args.text.clone(),
        from: args.from.clone(),
        to: args.to.clone(),
    };

    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("{}", i18n::err_runtime(e.to_string(), lang));
            return 1;
        }
    };

    let out = rt.block_on(async move { translate(&req, engine, lang).await });
    match out {
        Ok(out) => {
            if args.quiet {
                println!("{}", out.text);
            } else {
                match lang {
                    UiLang::Zh => {
                        println!("原文[{}]: {}", args.from, args.text);
                        println!("译文[{}]: {}", args.to, out.text);
                        println!("引擎: {}", out.engine);
                    }
                    UiLang::En => {
                        println!("Source[{}]: {}", args.from, args.text);
                        println!("Translation[{}]: {}", args.to, out.text);
                        println!("Engine: {}", out.engine);
                    }
                }
            }
            0
        }
        Err(e) => {
            eprintln!("{}", e.to_localized(lang));
            1
        }
    }
}
