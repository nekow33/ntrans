//! 翻译历史记录的持久化（JSON，新条目在前）。

use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::paths;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    /// 时间戳（毫秒）。
    pub ts: i64,
    /// 展示用本地时间串。
    pub at: String,
    pub from: String,
    pub to: String,
    pub source: String,
    pub translation: String,
    pub engine: String,
}

/// 简易历史仓库。
pub struct History {
    pub entries: Vec<HistoryEntry>,
    capacity: usize,
    dirty: bool,
}

impl History {
    pub fn load(capacity: usize) -> Self {
        let mut entries: Vec<HistoryEntry> = paths::load_json("history.json", Vec::new());
        entries.truncate(capacity);
        Self {
            entries,
            capacity: capacity.max(1),
            dirty: false,
        }
    }

    /// 纯内存历史（不读写磁盘），供测试使用。
    #[cfg(test)]
    pub fn fresh(capacity: usize) -> Self {
        Self {
            entries: Vec::new(),
            capacity: capacity.max(1),
            dirty: false,
        }
    }

    pub fn set_capacity(&mut self, capacity: usize) {
        let capacity = capacity.clamp(1, 10_000);
        if self.capacity == capacity {
            return;
        }
        self.capacity = capacity;
        self.entries.truncate(capacity);
        self.dirty = true;
    }

    /// 新增一条（相同原文+译文视为重复，移到最前）。
    pub fn add(
        &mut self,
        source: String,
        translation: String,
        from: String,
        to: String,
        engine: String,
    ) {
        if source.trim().is_empty() || translation.trim().is_empty() {
            return;
        }
        if let Some(pos) = self.entries.iter().position(|e| {
            e.source == source && e.translation == translation && e.from == from && e.to == to
        }) {
            let entry = self.entries.remove(pos);
            self.entries.insert(0, entry);
            self.dirty = true;
            return;
        }
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let at = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
        let id = format!("{ts}-{:x}", ts.unsigned_abs() ^ self.entries.len() as u64);
        self.entries.insert(
            0,
            HistoryEntry {
                id,
                ts,
                at,
                from,
                to,
                source,
                translation,
                engine,
            },
        );
        self.entries.truncate(self.capacity);
        self.dirty = true;
    }

    pub fn remove(&mut self, id: &str) {
        let before = self.entries.len();
        self.entries.retain(|e| e.id != id);
        self.dirty |= self.entries.len() != before;
    }

    pub fn clear(&mut self) {
        if !self.entries.is_empty() {
            self.entries.clear();
            self.dirty = true;
        }
    }

    /// 若有改动则写盘。
    pub fn flush(&mut self) {
        if !self.dirty {
            return;
        }
        if let Err(e) = paths::save_json("history.json", &self.entries) {
            eprintln!("保存历史失败: {e}");
        }
        self.dirty = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_dedup_and_cap() {
        let mut h = History {
            entries: Vec::new(),
            capacity: 3,
            dirty: false,
        };
        h.add(
            "a".into(),
            "一".into(),
            "en".into(),
            "zh-CN".into(),
            "google".into(),
        );
        h.add(
            "b".into(),
            "二".into(),
            "en".into(),
            "zh-CN".into(),
            "google".into(),
        );
        h.add(
            "c".into(),
            "三".into(),
            "en".into(),
            "zh-CN".into(),
            "google".into(),
        );
        h.add(
            "d".into(),
            "四".into(),
            "en".into(),
            "zh-CN".into(),
            "google".into(),
        );
        assert_eq!(h.entries.len(), 3);
        assert_eq!(h.entries[0].source, "d");
        h.add(
            "b".into(),
            "二".into(),
            "en".into(),
            "zh-CN".into(),
            "google".into(),
        );
        assert_eq!(h.entries.len(), 3);
        assert_eq!(h.entries[0].source, "b");
    }

    #[test]
    fn empty_ignored() {
        let mut h = History {
            entries: Vec::new(),
            capacity: 10,
            dirty: false,
        };
        h.add(
            "   ".into(),
            "".into(),
            "en".into(),
            "zh".into(),
            "x".into(),
        );
        assert!(h.entries.is_empty());
    }
}
