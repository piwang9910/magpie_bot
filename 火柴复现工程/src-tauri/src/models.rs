//! 数据模型：Rust 与前端共享的 IPC 契约（由 tauri-specta 生成 TS 类型）。
//! 对应 PLAN §3.3 / §3.6。

use serde::{Deserialize, Serialize};
use specta::Type;

/// 单条搜索/小程序结果的类型。`number` 为来电归属预留位（P2，仅占位）。
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ResultKind {
    App,
    Folder,
    File,
    Calc,
    Translate,
    /// 来电归属预留位（REQUIREMENTS §10.1，v1 不实现）
    Number,
}

/// 结果来源后端。`miniapp` 表示结果来自小程序处理器。
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ResultSource {
    Everything,
    Startmenu,
    Mdfind,
    Miniapp,
}

/// 统一的搜索结果卡片。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub id: String,
    pub kind: ResultKind,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub score: f64,
    pub source: ResultSource,
}

/// 查询请求。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Query {
    pub text: String,
    pub limit: u32,
}

/// 流式聊天消息的状态。
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MessageStatus {
    Streaming,
    Done,
    Aborted,
    Error,
}

/// 会话面板中的一条消息。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<MessageStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ChatRole {
    User,
    Assistant,
}

/// 热键后端选择。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum HotkeyBackendKind {
    /// 低级键盘钩子：双击 Ctrl（Windows）
    Hook,
    /// RegisterHotKey：Alt+Space（Windows）
    Reghotkey,
    /// macOS 平台后端（保留可迁移性）
    Mac,
}

/// 主题选择。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Theme {
    Light,
    Dark,
    System,
}

/// 应用设置。`number`/`clipboardPrefillOnWake`/`islandEnabled` 仅为预留位。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub hotkey_backend: HotkeyBackendKind,
    pub theme: Theme,
    pub max_results: u32,
    pub translate_triggers: Vec<String>,
    pub launcher_pins: Vec<String>,
    /// 默认 false；号码功能预留（REQUIREMENTS §10.1）
    pub clipboard_prefill_on_wake: bool,
    /// 常驻灵动岛态开关，默认 false（PLAN §3.7 / ADR-7）
    pub island_enabled: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            hotkey_backend: default_hotkey_backend(),
            theme: Theme::System,
            max_results: 20,
            translate_triggers: vec!["fy".into(), "翻译".into()],
            launcher_pins: Vec::new(),
            clipboard_prefill_on_wake: false,
            island_enabled: false,
        }
    }
}

/// 按目标平台选择默认热键后端：macOS 上用 MacBackend，其余用 Hook（双击 Ctrl）。
pub fn default_hotkey_backend() -> HotkeyBackendKind {
    #[cfg(target_os = "macos")]
    {
        HotkeyBackendKind::Mac
    }
    #[cfg(not(target_os = "macos"))]
    {
        HotkeyBackendKind::Hook
    }
}
