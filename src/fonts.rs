//! 跨平台加载系统中文字体，供 egui 渲染 CJK 文本。
//!
//! egui 自带字体不含中文字形，这里从操作系统常见路径探测一个可用的
//! CJK 字体文件并注入字体库（放在拉丁字体之后作为回退）。
//! 支持 `.ttf/.otf/.ttc`（ttc 通过索引 0 读取）。

use egui::{FontData, FontDefinitions, FontFamily};

/// 各平台常见中文字体候选路径（按优先级）。
#[rustfmt::skip]
const CANDIDATES: &[&str] = &[
    // Linux / FreeBSD
    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/opentype/noto/NotoSansCJKsc-Regular.otf",
    "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
    "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
    "/usr/share/fonts/wenquanyi/wqy-zenhei/wqy-zenhei.ttc",
    "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
    "/usr/share/fonts/TTF/uming.ttc",
    "/usr/share/fonts/TTF/ukai.ttc",
    // macOS
    "/System/Library/Fonts/PingFang.ttc",
    "/System/Library/Fonts/STHeiti Light.ttc",
    "/System/Library/Fonts/Hiragino Sans GB.ttc",
    // Windows
    "C:\\Windows\\Fonts\\msyh.ttc",
    "C:\\Windows\\Fonts\\msyh.ttf",
    "C:\\Windows\\Fonts\\simhei.ttf",
    "C:\\Windows\\Fonts\\simsun.ttc",
    "C:\\Windows\\Fonts\\Deng.ttf",
];

/// 返回第一个存在且可读的候选字体字节。
fn load_cjk_bytes() -> Option<Vec<u8>> {
    for path in CANDIDATES {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        if bytes.len() >= 4 {
            return Some(bytes);
        }
    }
    None
}

/// 安装 CJK 字体到 egui 上下文（追加到各字族末尾作回退）。
/// 返回是否成功安装。
pub fn install_cjk(ctx: &egui::Context) -> bool {
    let Some(bytes) = load_cjk_bytes() else {
        eprintln!("警告：未找到系统 CJK 字体，中文可能显示为方块");
        return false;
    };
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "system-cjk".into(),
        std::sync::Arc::new(FontData::from_owned(bytes)),
    );
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .push("system-cjk".into());
    }
    ctx.set_fonts(fonts);
    true
}
