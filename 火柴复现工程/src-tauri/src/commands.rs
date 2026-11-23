//! Tauri commands：按领域聚合（search / launch / system / app），不为每个操作开一个 command（PLAN §3.6 薄桥接层）。
//! M0 暴露一组最小 command 用于实测 capability 五步清单（PLAN §3.11）。

use tauri::State;

use crate::gateway::CancelToken;
use crate::models::{AppSettings, SearchResult};
use crate::settings::SettingsStore;
use crate::search::{self, SearchBackend};
use crate::window;

pub mod app;

/// 最小冒烟 command：返回一条固定文本。
/// M0 用它实测"应用自有 command 是否需在 capability 显式声明"。
#[tauri::command]
#[specta::specta]
pub fn greet(name: &str) -> String {
    format!("你好，{}！喜鹊助手骨架已就绪。", name)
}

/// 读取当前设置。
#[tauri::command]
#[specta::specta]
pub fn get_settings(store: State<'_, SettingsStore>) -> Result<AppSettings, String> {
    Ok(store.load())
}

/// 保存设置。
#[tauri::command]
#[specta::specta]
pub fn save_settings(
    settings: AppSettings,
    store: State<'_, SettingsStore>,
) -> Result<(), String> {
    store.save(&settings).map_err(|e| e.to_string())
}

/// 占位搜索：当前返回空列表，验证 IPC 契约贯通（真正实现在 M2）。
#[tauri::command]
#[specta::specta]
pub fn search_placeholder(query: crate::models::Query) -> Result<Vec<SearchResult>, String> {
    let backend = search::default_backend();
    backend.search(&query).map_err(|e| e.to_string())
}

/// 取消当前 AI 流式请求（占位：M3 接入真实取消令牌管理）。
#[tauri::command]
#[specta::specta]
pub fn cancel_generation() -> Result<(), String> {
    let _token = CancelToken::new();
    Ok(())
}

/// 所有 command 的集合，供 tauri-specta 生成类型与注册 invoke_handler。
pub fn collect_commands() -> tauri_specta::Commands<tauri::Wry> {
    tauri_specta::collect_commands![
        greet,
        get_settings,
        save_settings,
        search_placeholder,
        cancel_generation,
        app::open_panel,
        app::hide_hud,
        app::hide_panel,
        app::focus_hud,
    ]
}

/// 创建 tauri-specta builder：注册 command 与所有需要生成 TS 类型的结构体。
pub fn specta_builder() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::<tauri::Wry>::new()
        .commands(collect_commands())
        .typ::<AppSettings>()
        .typ::<SearchResult>()
        .typ::<crate::models::Query>()
        .typ::<crate::models::ChatMessage>()
}

/// 保留 window 模块引用（commands 经 app 子模块间接使用）。
#[allow(dead_code)]
fn _assert_window(_: fn(&tauri::AppHandle)) {
    let _ = window::show_hud;
}
