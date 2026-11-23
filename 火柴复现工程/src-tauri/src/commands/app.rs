//! 窗口相关 command（持有 AppHandle，需单独定义）。

use tauri::AppHandle;

/// 从 HUD 按 Enter 进入 AI 时打开会话面板。
#[tauri::command]
#[specta::specta]
pub fn open_panel(app: AppHandle) {
    super::window::show_panel(&app);
    super::window::hide_hud(&app);
}

/// 隐藏 HUD（Esc / 失焦）。
#[tauri::command]
#[specta::specta]
pub fn hide_hud(app: AppHandle) {
    super::window::hide_hud(&app);
}

/// 关闭面板并把焦点交还 HUD（Esc）。
#[tauri::command]
#[specta::specta]
pub fn hide_panel(app: AppHandle) {
    super::window::hide_panel(&app);
}

/// 面板切回 HUD（Ctrl+Tab）。
#[tauri::command]
#[specta::specta]
pub fn focus_hud(app: AppHandle) {
    super::window::show_hud(&app);
}
