//! 跨平台应用数据目录定位。

use std::path::PathBuf;

/// 应用数据根目录（历史、配置都放这里）。
///
/// Windows: `%APPDATA%\ntrans`
/// macOS:   `~/Library/Application Support/ntrans`
/// Linux:   `$XDG_DATA_HOME/ntrans` 或 `~/.local/share/ntrans`
pub fn data_dir() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("ntrans")
}

/// 确保数据目录存在，并返回该目录。
pub fn ensure_data_dir() -> std::io::Result<PathBuf> {
    let dir = data_dir();
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// 从 JSON 文件加载（不存在或损坏则返回默认值）。
pub fn load_json<T: serde::de::DeserializeOwned>(name: &str, default: T) -> T {
    let path = data_dir().join(name);
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or(default),
        Err(_) => default,
    }
}

/// 保存 JSON 文件。
pub fn save_json<T: serde::Serialize>(name: &str, value: &T) -> std::io::Result<()> {
    let dir = ensure_data_dir()?;
    let path = dir.join(name);
    let content = serde_json::to_string_pretty(value).map_err(std::io::Error::other)?;
    std::fs::write(path, content)
}
