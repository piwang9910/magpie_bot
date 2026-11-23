//! 设置读写：内存态 + 本地 JSON 文件持久化。
//! M0 只实现读写骨架与默认值；设置页 UI 属 M5。

use std::path::PathBuf;

use crate::models::AppSettings;

/// 设置文件名（放在应用配置目录，跨平台用 dirs 风格的环境变量兜底）。
const SETTINGS_FILE: &str = "settings.json";

/// 设置仓储：封装设置文件路径，避免散落的路径拼接。
#[derive(Debug, Clone)]
pub struct SettingsStore {
    path: PathBuf,
}

impl SettingsStore {
    pub fn new(dir: PathBuf) -> Self {
        Self {
            path: dir.join(SETTINGS_FILE),
        }
    }

    /// 读取设置；文件不存在或损坏时回退到默认值。
    pub fn load(&self) -> AppSettings {
        match std::fs::read_to_string(&self.path) {
            Ok(text) => serde_json::from_str::<AppSettings>(&text).unwrap_or_default(),
            Err(_) => AppSettings::default(),
        }
    }

    /// 写入设置；父目录不存在时自动创建。
    pub fn save(&self, settings: &AppSettings) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = serde_json::to_string_pretty(settings)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(&self.path, text)
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

/// 默认设置目录：优先用平台配置目录的环境变量；
/// macOS 用 ~/Library/Application Support，Windows 用 %APPDATA%。
pub fn default_config_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return PathBuf::from(appdata).join("magpie-bot");
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("magpie-bot");
        }
    }
    // 兜底：当前目录
    PathBuf::from(".")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{HotkeyBackendKind, Theme};

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("magpie-settings-{}", name));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn load_returns_default_when_file_missing() {
        let store = SettingsStore::new(temp_dir("missing"));
        let s = store.load();
        assert_eq!(s.max_results, 20);
        assert_eq!(s.theme, Theme::System);
        assert!(!s.island_enabled);
        assert!(!s.clipboard_prefill_on_wake);
        assert_eq!(s.translate_triggers, vec!["fy", "翻译"]);
    }

    #[test]
    fn save_then_load_roundtrip() {
        let dir = temp_dir("roundtrip");
        let store = SettingsStore::new(dir.clone());
        let mut s = AppSettings::default();
        s.theme = Theme::Dark;
        s.island_enabled = true;
        s.hotkey_backend = HotkeyBackendKind::Reghotkey;
        store.save(&s).unwrap();
        let loaded = store.load();
        assert_eq!(loaded.theme, Theme::Dark);
        assert!(loaded.island_enabled);
        assert_eq!(loaded.hotkey_backend, HotkeyBackendKind::Reghotkey);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn load_returns_default_on_corrupt_file() {
        let dir = temp_dir("corrupt");
        let store = SettingsStore::new(dir.clone());
        std::fs::write(dir.join(SETTINGS_FILE), "not json").unwrap();
        let s = store.load();
        assert_eq!(s.max_results, 20);
        let _ = std::fs::remove_dir_all(dir);
    }
}
