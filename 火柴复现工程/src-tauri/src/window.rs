//! 窗口生命周期、多屏定位、avoidTop 常量（PLAN §3.7 分区契约）。
//! 平台专属代码集中在此文件的 cfg(windows) 分支（PLAN §3.5）。

use tauri::{Manager, WebviewWindow};

/// 顶部安全距：避免唤出时遮住顶部信息岛（路 B 分区契约）。
pub const AVOID_TOP_PT: f64 = 140.0;

/// 常驻灵动岛窗口 label（框架位，功能后续实现，ADR-7）。
pub const ISLAND_WINDOW_LABEL: &str = "island";

/// 唤出 HUD 并聚焦输入框。
pub fn show_hud(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("hud") {
        position_hud(&window);
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// 隐藏 HUD。
pub fn hide_hud(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("hud") {
        let _ = window.hide();
    }
}

/// 显示会话面板。初始问题经窗口启动参数传递（M3 接入）。
pub fn show_panel(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("panel") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// 隐藏会话面板并把焦点交还 HUD。
pub fn hide_panel(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("panel") {
        let _ = window.hide();
    }
    show_hud(app);
}

/// 按鼠标所在屏幕定位 HUD，并应用 avoidTop 安全距。
fn position_hud(window: &WebviewWindow) {
    // M0 只应用顶部安全距常量；多屏跟随在 M1 实现。
    #[cfg(target_os = "windows")]
    {
        // Windows 专属：获取当前光标所在显示器与 DPI，计算物理像素偏移。
        // M0 仅记录常量，实际定位待 M1。
        tracing::debug!(avoid_top_pt = AVOID_TOP_PT, "Windows HUD 定位（骨架）");
    }
    let _ = window;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avoid_top_is_140pt() {
        assert_eq!(AVOID_TOP_PT, 140.0);
    }
}
